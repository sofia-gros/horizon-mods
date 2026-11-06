#!/usr/bin/env python3
"""Exact pure API authoring composition; no host, transport or gameplay claim."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
FLOOR = 10 * 1024**3 + 128 * 1024**2

def existing_parent(path):
    path = Path(path).expanduser().resolve()
    while not path.exists():
        if path.parent == path: raise ValueError('No existing output filesystem: ' + str(path))
        path = path.parent
    return path

def write_filesystems(work, env):
    # apt writes /; rustup may use a different mount from workspace and Cargo.
    return [existing_parent(p) for p in [ROOT, work, env['CARGO_HOME'], Path('/'),
            env.get('RUSTUP_HOME', str(Path.home()/'.rustup'))]]

def sha(data): return hashlib.sha256(data).hexdigest()
def read_inputs(): return json.loads((HERE / 'inputs.json').read_text())
def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
def seal(root, tracked=False):
    paths = ([root / p for p in git(root, 'ls-files').splitlines()] if tracked else
             [p for p in root.rglob('*') if p.is_file() and '.git' not in p.relative_to(root).parts])
    return {str(p.relative_to(root)): sha(p.read_bytes()) for p in sorted(paths)}
def assert_seal(root, expected, tracked=False):
    if seal(root, tracked) != expected: raise ValueError('Source/lock/file-set drift: ' + str(root))
def out_dirs(target):
    return {str(p.relative_to(target)): sha(p.read_bytes())
            for out in sorted(target.glob('debug/build/*/out')) for p in sorted(out.rglob('*')) if p.is_file()}

class Runner:
    def __init__(self, work, env):
        self.work, self.env = work, env
        self.minimum_free = None
    def guard(self, reserve=0):
        free = min(shutil.disk_usage(p).free for p in write_filesystems(self.work, self.env))
        self.minimum_free = free if self.minimum_free is None else min(self.minimum_free, free)
        if free < FLOOR + reserve: raise RuntimeError('Disk floor reached; own process group stopped')
    def run(self, name, command, cwd=None, timeout=900):
        self.guard()
        path = self.work / (name + '.log')
        print('PHASE', name, json.dumps(command), flush=True)
        started = time.monotonic()
        with path.open('w') as log:
            proc = subprocess.Popen(command, cwd=cwd or ROOT, env=self.env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                while proc.poll() is None:
                    self.guard()
                    if time.monotonic() - started > timeout: raise TimeoutError(name)
                    time.sleep(0.2)
            finally:
                # Never leave a child behind merely because the direct leader exited.
                # TERM has one bounded second; KILL and reap are bounded separately.
                if proc.poll() is None:
                    try: os.killpg(proc.pid, signal.SIGTERM)
                    except ProcessLookupError: pass
                    deadline = time.monotonic() + 1.0
                    while proc.poll() is None and time.monotonic() < deadline:
                        time.sleep(0.05)
                try: os.killpg(proc.pid, signal.SIGKILL)
                except ProcessLookupError: pass
                proc.wait(timeout=2)
                deadline = time.monotonic() + 1.0
                while True:
                    try: os.killpg(proc.pid, 0)
                    except ProcessLookupError: break
                    if time.monotonic() >= deadline: raise RuntimeError('Own process group remains: ' + name)
                    time.sleep(0.02)
                print(path.read_text(errors='replace'), end='', flush=True)
        self.guard()
        print('TERMINAL', name, proc.returncode, 'seconds', round(time.monotonic()-started, 3), flush=True)
        if proc.returncode != 0: raise RuntimeError(name + ' exited ' + str(proc.returncode))
        return path.read_text()

def validate_tests(log, names, filtered=None):
    results = re.findall(r'^test (\S+) \.\.\. (\w+)\s*$', log, re.M)
    if len(results) != len(names) or {name for name, _ in results} != set(names):
        raise ValueError('Missing/duplicate/unexpected test names')
    if any(result != 'ok' for _, result in results): raise ValueError('A required test did not pass')
    summaries = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', log, re.M)
    if len(summaries) != 1 or summaries[0][:4] != (str(len(names)),'0','0','0'):
        raise ValueError('Result/count mismatch or skipped tests')
    if filtered is not None and summaries[0][4] != str(filtered): raise ValueError('Unexpected filtered test count')


def actual_compile(log, crate, test=False):
    for line in log.splitlines():
        if 'Running `' in line and 'rustc' in line and '--crate-name '+crate+' ' in line:
            if not test or ' --test' in line: return
    raise ValueError('Missing actual compiler evidence: '+crate)

def validate_suite(log, count, required):
    rows=re.findall(r'^test (\S+) \.\.\. (\w+)\s*$',log,re.M)
    names=[n for n,_ in rows]
    if len(names)!=count or len(set(names))!=count or not set(required)<=set(names):
        raise ValueError('Unexpected test names/count or omitted new cases')
    if any(state!='ok' for _,state in rows):raise ValueError('Nonpassing test')
    summaries=re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',log,re.M)
    if summaries!=[(str(count),'0','0','0','0')]:raise ValueError('Unexpected summary/skip/filter')
    return sorted(names)

def regular_seal(root):
    import stat
    result={}
    if root.is_symlink() or not root.is_dir():raise ValueError('Source directory is not regular')
    for p in root.rglob('*'):
        mode=p.lstat().st_mode
        if stat.S_ISDIR(mode):continue
        if not stat.S_ISREG(mode):raise ValueError('Symlink/special source: '+str(p))
        result[p.relative_to(root).as_posix()]=sha(p.read_bytes())
    return result

def identities(lock):
    import tomllib
    return {(p['name'],p['version'],p.get('source'),p.get('checksum')) for p in tomllib.loads(lock.read_text())['package']}

def assert_metadata(metadata, paths):
    local=[p for p in metadata['packages'] if p.get('source') is None]
    observed={p['name']:str(Path(p['manifest_path']).resolve()) for p in local}
    expected={name:str((path/'Cargo.toml').resolve()) for name,path in paths.items()}
    if len(local)!=len(paths) or observed!=expected:raise ValueError('Duplicate/substitute local API package')

def test_after_compile(runner, name, command, count, required):
    before=out_dirs(runner.work/'target')
    output=runner.run(name,command)
    if any('Running `' in line and 'rustc' in line for line in output.splitlines()):
        raise ValueError('Unexpected recompilation during sealed execution: '+name)
    if out_dirs(runner.work/'target')!=before:raise ValueError('Generated inputs changed during '+name)
    return validate_suite(output,count,required)

def main():
    config=read_inputs()
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if (os.environ.get('GITHUB_ACTIONS')!='true' or os.environ.get('GITHUB_REPOSITORY')!=config['repository'] or
        os.environ.get('GITHUB_EVENT_NAME')!='push' or os.environ.get('GITHUB_REF')!='refs/heads/'+config['branch'] or
        event.get('repository',{}).get('private') is not False or
        git(ROOT,'rev-parse','HEAD')!=os.environ.get('GITHUB_SHA') or git(ROOT,'status','--porcelain','--untracked-files=all')):
        raise ValueError('Require clean exact authorized public CI branch')
    for path,digest in config['inputs_sha256'].items():
        if sha((ROOT/path).read_bytes())!=digest:raise ValueError('Frozen input mismatch: '+path)
    for spec in config['apis'].values():
        if regular_seal(ROOT/spec['path'])!=spec['files']:raise ValueError('API source/file-set mismatch')
    original=seal(ROOT,True)
    work=Path(os.environ['RUNNER_TEMP'])/('author-api-'+os.environ['GITHUB_SHA'])
    env=os.environ.copy()
    for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:env.pop(key,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'target'),
               CARGO_BUILD_JOBS='1',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',
               CARGO_TERM_COLOR='never',TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1')
    if min(shutil.disk_usage(p).free for p in write_filesystems(work,env))<FLOOR+2*1024**3:
        raise RuntimeError('Need disk floor plus 2 GiB setup reserve')
    work.mkdir();Path(env['CARGO_HOME']).mkdir();Path(env['TMPDIR']).mkdir()
    runner=Runner(work,env);gates={};snapshots={};api_dirs={};error=None;verified=False;generated=None
    try:
        checklog=runner.run('runner-checks',[sys.executable,str(HERE/'test_runner.py'),'-v'])
        if re.findall(r'^Ran (\d+) tests in .+$',checklog,re.M)!=['13'] or re.findall(r'^OK$',checklog,re.M)!=['OK']:raise ValueError('Expected 13 unskipped runner checks')
        gates['runner13']={'passed':True,'tests':13}
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal'],timeout=600)
        checkout=work/'official-lock-source';pin=config['official']
        runner.run('official-init',['git','init',str(checkout)])
        runner.run('official-origin',['git','-C',str(checkout),'remote','add','origin',pin['url']])
        runner.run('official-fetch',['git','-C',str(checkout),'fetch','--depth','1','origin',pin['revision']])
        if git(checkout,'rev-parse','FETCH_HEAD')!=pin['revision'] or git(checkout,'remote','get-url','origin')!=pin['url']:
            raise ValueError('Wrong official lock provenance')
        lock_text=runner.run('official-lock',['git','-C',str(checkout),'show',pin['revision']+':Cargo.lock'])
        official_lock=work/'official.Cargo.lock';official_lock.write_text(lock_text)
        if sha(official_lock.read_bytes())!=pin['lock_sha256']:raise ValueError('Official lock mismatch')
        official_ids=identities(official_lock)
        for key,spec in config['apis'].items():
            dst=work/key;shutil.copytree(ROOT/spec['path'],dst)
            shutil.copyfile(ROOT/spec['lock'],dst/'Cargo.lock')
            if identities(dst/'Cargo.lock')-official_ids-{(spec['package'],'0.1.0',None,None)}:
                raise ValueError('API dependency lock escapes official identities')
            snapshots[str(dst)]=regular_seal(dst);api_dirs[spec['package']]=dst
            runner.run(key+'-fetch',['cargo','fetch','--locked','--manifest-path',str(dst/'Cargo.toml')])
            command=['cargo','test','--lib','--locked','--offline','--manifest-path',str(dst/'Cargo.toml'),'-vv']
            built=runner.run(key+'-compile',command+['--no-run'])
            actual_compile(built,spec['crate'],True)
            names=test_after_compile(runner,key+'-tests',command+['--','--test-threads=1'],spec['tests'],spec['required_tests'])
            gates[key]={'passed':True,'tests':len(names),'names':names}
        inventory=api_dirs['pluto-inventory-api'];public=api_dirs['pluto-api']
        guidecmd=['cargo','test','--example','guide_observer','--locked','--offline','--manifest-path',str(inventory/'Cargo.toml'),'-vv']
        built=runner.run('guide-example-compile',guidecmd+['--no-run']);actual_compile(built,'guide_observer',True)
        test_after_compile(runner,'guide-example',guidecmd+['--','--test-threads=1'],1,['public_startup_and_content_example']);gates['guide_example']={'passed':True,'tests':1}
        consumer=work/'consumer';consumer.mkdir();(consumer/'src').mkdir();(consumer/'tests').mkdir()
        manifest='[package]\nname="pluto-authoring-examples"\nversion="0.1.0"\nedition="2021"\nautotests=false\n\n[workspace]\n\n[dependencies]\n'
        for name,path in api_dirs.items():manifest+=name+' = { path = '+json.dumps(str(path))+' }\n'
        for key,path in config['consumer_bins'].items():
            dest=consumer/'src'/(key+'.rs');shutil.copyfile(ROOT/path,dest)
            manifest+='\n[[bin]]\nname='+json.dumps(key)+'\npath='+json.dumps('src/'+key+'.rs')+'\n'
        manifest+='\n[[test]]\nname="fluid_usage"\npath="tests/fluid_usage.rs"\n'
        (consumer/'Cargo.toml').write_text(manifest)
        shutil.copyfile(ROOT/config['fluid_example'],consumer/'tests/fluid_example_source.rs')
        shutil.copyfile(HERE/'fluid_usage.rs',consumer/'tests/fluid_usage.rs')
        # Exact seeded registry graph, with only these two real path packages and this consumer.
        seed=(ROOT/config['apis']['inventory']['lock']).read_text()
        seed+='\n[[package]]\nname = "pluto-api"\nversion = "0.1.0"\ndependencies = ["serde_json"]\n'
        seed+='\n[[package]]\nname = "pluto-authoring-examples"\nversion = "0.1.0"\ndependencies = ["pluto-api", "pluto-inventory-api"]\n'
        (consumer/'Cargo.lock').write_text(seed)
        if identities(consumer/'Cargo.lock')-official_ids-{(n,'0.1.0',None,None) for n in [*api_dirs,'pluto-authoring-examples']}:
            raise ValueError('Example lock escapes exact API/official graph')
        snapshots[str(consumer)]=regular_seal(consumer)
        metadata=runner.run('example-metadata',['cargo','metadata','--locked','--offline','--format-version','1','--manifest-path',str(consumer/'Cargo.toml')])
        assert_metadata(json.loads(metadata),{**api_dirs,'pluto-authoring-examples':consumer})
        typed=runner.run('recipe-storage-type',['cargo','check','--bins','--locked','--offline','--manifest-path',str(consumer/'Cargo.toml'),'-vv'])
        for name in config['consumer_bins']:actual_compile(typed,name)
        gates['recipe_storage_examples_type']={'passed':True,'bins':list(config['consumer_bins'])}
        built=runner.run('storage-example-build',['cargo','build','--bin','storage','--locked','--offline','--manifest-path',str(consumer/'Cargo.toml'),'-vv'])
        actual_compile(built,'storage')
        before=out_dirs(work/'target');runner.run('storage-example',[str(work/'target/debug/storage')])
        if out_dirs(work/'target')!=before:raise ValueError('Generated inputs changed during storage example')
        gates['storage_example_run']={'passed':True,'scope':'six assertions in the unchanged source; no commit or event'}
        fluidcmd=['cargo','test','--test','fluid_usage','--locked','--offline','--manifest-path',str(consumer/'Cargo.toml'),'-vv']
        built=runner.run('fluid-example-compile',fluidcmd+['--no-run']);actual_compile(built,'fluid_usage',True)
        test_after_compile(runner,'fluid-example',fluidcmd+['--','--test-threads=1'],1,['actual_example_preserves_unknown_and_registration_contract']);gates['fluid_example']={'passed':True,'tests':1}
        generated=out_dirs(work/'target')
        if set(gates)!={'runner13','inventory','public','guide_example','recipe_storage_examples_type','storage_example_run','fluid_example'}:raise ValueError('Missing required gate')
    except BaseException as exc:
        error=type(exc).__name__+': '+str(exc)
    finally:
        try:
            assert_seal(ROOT,original,True)
            if git(ROOT,'status','--porcelain','--untracked-files=all'):raise ValueError('Dirty/untracked checkout after proof')
            for path,before in snapshots.items():
                if regular_seal(Path(path))!=before:raise ValueError('API/example source or lock changed: '+path)
            runner.guard();verified=True
        except BaseException as exc:
            error=(error+'; ' if error else '')+'verification: '+type(exc).__name__+': '+str(exc)
        report={'head':os.environ['GITHUB_SHA'],'passed':error is None and verified,'error':error,'gates':gates,
                'minimum_free_bytes':runner.minimum_free,'floor_bytes':FLOOR,'input_cutoffs':config['cutoffs'],
                'source_seal_sha256':sha(json.dumps(original,sort_keys=True).encode()),
                'copied_input_seals':{p:sha(json.dumps(s,sort_keys=True).encode()) for p,s in snapshots.items()},
                'generated_output_seal_sha256':sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None,
                'scope':'Pure API wrappers and examples only. Mixed generic API source composition; native adapters are NOT composed or verified. No Game/Font/HUD/GPU/LAN/contact or consumer activation.'}
        print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:f.write('## Author API proof\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    if error is not None:raise SystemExit(1)

if __name__=='__main__':main()
