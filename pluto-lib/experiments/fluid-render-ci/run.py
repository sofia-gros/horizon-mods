#!/usr/bin/env python3
"""Bounded source/type and material-contract gates; not a rendered-fluid proof."""
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
    def run(self, name, command, cwd=None, timeout=2400):
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

def native_rustc(log, names):
    for name in names:
        if not any('Running `' in line and 'rustc' in line and '--crate-name '+name+' ' in line for line in log.splitlines()):
            raise ValueError('Missing actual compilation invocation: ' + name)

def native_host_checks(log):
    native_rustc(log,['pluto_api','horizon_client_render','horizon_client_lib','horizon_client'])
    for path in ['src/lib.rs','src/main.rs']:
        if not any('Running `' in line and 'rustc' in line and '--crate-name horizon_client ' in line and path in line for line in log.splitlines()):
            raise ValueError('Missing actual host target check: '+path)

def validate_source_checks(log):
    summaries=re.findall(r'^Ran (\d+) tests? in .+$',log,re.M)
    successes=re.findall(r'^test_\S+ .* \.\.\. ok$',log,re.M)
    if summaries!=['19'] or len(successes)!=19 or re.findall(r'^OK$',log,re.M)!=['OK']:
        raise ValueError('Expected exactly19 executed passing source guards')

def export(source, revision, destination):
    import importlib.util
    spec=importlib.util.spec_from_file_location('portable',ROOT/'tconstruct/tools/portable_archive.py')
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    destination.mkdir()
    proc=subprocess.Popen(['git','-C',str(source),'archive','--format=tar',revision],stdout=subprocess.PIPE)
    try:
        module.extract_git_tar(proc.stdout,destination)
        if proc.wait(timeout=60):raise ValueError('Official export failed')
    finally:
        proc.stdout.close()
        if proc.poll() is None:proc.terminate();proc.wait(timeout=2)
    if revision==read_inputs()['official']['horizon']['revision']:
        (destination/'.horizon-revision').write_text(revision+'\n')

def unpack(archive, destination):
    import zipfile
    destination.mkdir()
    with zipfile.ZipFile(archive) as z:
        for entry in z.infolist():
            p=Path(entry.filename)
            if p.is_absolute() or '..' in p.parts or not p.parts or ((entry.external_attr>>16)&0o170000)!=0o100000:
                raise ValueError('Unexpected package member')
            target=destination/p;target.parent.mkdir(parents=True,exist_ok=True)
            with target.open('xb') as f:f.write(z.read(entry))

def main():
    config=read_inputs()
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if (os.environ.get('GITHUB_ACTIONS')!='true' or os.environ.get('GITHUB_REPOSITORY')!=config['repository'] or
        os.environ.get('GITHUB_EVENT_NAME')!='push' or os.environ.get('GITHUB_REF')!='refs/heads/'+config['branch'] or
        event.get('repository',{}).get('private') is not False or
        git(ROOT,'rev-parse','HEAD')!=os.environ.get('GITHUB_SHA') or git(ROOT,'status','--porcelain','--untracked-files=all')):
        raise ValueError('Require clean exact authorized public CI branch')
    for path,digest in config['source_sha256'].items():
        if sha((ROOT/path).read_bytes())!=digest:raise ValueError('Frozen input mismatch: '+path)
    before_mod=seal(ROOT,True)
    work=Path(os.environ['RUNNER_TEMP'])/('fluid-render-'+os.environ['GITHUB_SHA'])
    env=os.environ.copy()
    for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:env.pop(key,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'host-target'),
               CARGO_BUILD_JOBS='1',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_RELEASE_DEBUG='0',
               CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1')
    if min(shutil.disk_usage(p).free for p in write_filesystems(work,env))<FLOOR:raise RuntimeError('Preflight disk floor')
    work.mkdir();Path(env['CARGO_HOME']).mkdir();Path(env['TMPDIR']).mkdir()
    runner=Runner(work,env);gates={};snapshots={};stage=None;stage_before=None;generated=None
    native=ROOT/'pluto-lib/mixins/pluto_native_creative'
    try:
        runner.guard(reserve=2*1024**3)
        runner.run('apt-update',['sudo','apt-get','update'],timeout=600)
        runner.run('apt-install',['sudo','apt-get','install','-y','--no-install-recommends','libasound2-dev','libudev-dev'],timeout=600)
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal','--component','rustfmt'],timeout=600)
        source_result=runner.run('source-guards',['python3','-m','unittest','discover','-s','pluto-lib/tests','-p','test_terrain*.py','-v'])
        validate_source_checks(source_result);gates['source19']='passed'
        # Pure production modules, not a copied/modelled renderer or native owner.
        std=work/'std';std.mkdir()
        lines=[]
        for name,path in config['std_modules'].items():
            shutil.copyfile(native/path,std/path)
            lines.append('#[path='+json.dumps(path)+'] mod '+name+';')
        (std/'tests.rs').write_text('\n'.join(lines)+'\n');std_seal=seal(std)
        runner.run('std-compile',['rustc','--edition=2024','--test','-C','debuginfo=0','-C','opt-level=0',str(std/'tests.rs'),'-o',str(work/'std-tests')])
        result=runner.run('std30',[str(work/'std-tests'),'--test-threads=1']);validate_tests(result,config['std_tests'],0);assert_seal(std,std_seal);gates['std30']='passed'
        for name,info in config['official'].items():
            checkout=work/(name+'-git');original=work/name
            runner.run(name+'-init',['git','init',str(checkout)])
            runner.run(name+'-origin',['git','-C',str(checkout),'remote','add','origin',info['url']])
            runner.run(name+'-fetch',['git','-C',str(checkout),'fetch','--depth','1','origin',info['revision']])
            runner.run(name+'-checkout',['git','-C',str(checkout),'checkout','--detach','FETCH_HEAD'])
            if git(checkout,'rev-parse','HEAD')!=info['revision'] or git(checkout,'remote','get-url','origin')!=info['url'] or git(checkout,'status','--porcelain','--untracked-files=all'):
                raise ValueError('Official source identity mismatch')
            runner.run(name+'-export',['python3',str(HERE/'run.py'),'--export',str(checkout),info['revision'],str(original)])
            snapshots[str(original)]=seal(original)
        official=work/'horizon';mixins=work/'mixins'
        if sha((official/'Cargo.lock').read_bytes())!=config['official_lock_sha256']:raise ValueError('Official Cargo.lock changed')
        # Real dependency-backed public API, seeded from the source-frozen lock.
        api=work/'api';shutil.copytree(native/'api',api,ignore=shutil.ignore_patterns('target','Cargo.lock'))
        shutil.copyfile(ROOT/'pluto-lib/tests/fluid-api.Cargo.lock',api/'Cargo.lock')
        import tomllib
        def identities(lock):return {(p['name'],p['version'],p.get('source'),p.get('checksum')) for p in tomllib.loads(lock.read_text())['package']}
        if identities(api/'Cargo.lock')-identities(official/'Cargo.lock')-{('pluto-api','0.1.0',None,None)}:raise ValueError('API lock escapes official package identities')
        api_seal=seal(api);snapshots[str(api)]=api_seal
        runner.run('api-fetch',['cargo','fetch','--locked','--manifest-path',str(api/'Cargo.toml')])
        result=runner.run('fluid21',['cargo','test','--offline','--locked','--manifest-path',str(api/'Cargo.toml'),'--target-dir',str(work/'api-target'),'--lib','fluid::tests::','--','--test-threads=1'])
        validate_tests(result,config['fluid_tests']);assert_seal(api,api_seal);gates['fluid21']='passed'
        runner.run('mixins-build',['cargo','build','--release','--locked','-p','horizon-mixins-cli','--target-dir',str(work/'mixins-target')],cwd=mixins)
        cli=work/'mixins-target/release/horizon-mixins';bundle=work/'plutolib.hnmod';mods=work/'package'
        runner.run('package',['python3',str(ROOT/'pluto-lib/tools/package_native.py'),str(bundle)])
        if sha(bundle.read_bytes())!=config['bundle_sha256']:raise ValueError('Frozen production bundle mismatch')
        runner.run('unpack',['python3',str(HERE/'run.py'),'--unpack',str(bundle),str(mods)])
        snapshots[str(mods)]=seal(mods)
        plan=work/'plan.json';stage=work/'stage'
        runner.run('official-plan',[str(cli),'plan','--workspace',str(official),'--mods',str(mods/'mixins'),'--out',str(plan)])
        gates['plan']='passed'
        runner.run('all12guards',['python3',str(ROOT/'pluto-lib/tools/check_native_plan.py'),str(plan),str(official)])
        gates['complete_plan_guards']='passed'
        runner.run('official-apply',[str(cli),'apply','--workspace',str(official),'--mods',str(mods/'mixins'),'--out',str(stage)])
        stage_before=seal(stage);gates['apply']='passed'
        runner.run('stage-locked-fetch',['cargo','fetch','--locked','--manifest-path',str(stage/'Cargo.toml')])
        checked=runner.run('host-type',['cargo','check','--offline','--locked','--manifest-path',str(stage/'Cargo.toml'),'-p','horizon-client','--no-default-features','--lib','--bin','horizon-client','-vv'],cwd=stage)
        native_host_checks(checked)
        assert_seal(stage,stage_before);gates['host_lib_bin_type']='passed'
        testcmd=['cargo','test','--offline','--locked','--manifest-path',str(stage/'Cargo.toml'),'-p','horizon-client-render','--no-default-features','--lib']
        built=runner.run('native-material-compile',testcmd+['--no-run','-vv'],cwd=stage);native_rustc(built,['horizon_client_render'])
        generated=out_dirs(work/'host-target')
        result=runner.run('native-material2',testcmd+['mesher::pluto_terrain_cell_host::material_contract_tests::','--','--test-threads=1'],cwd=stage)
        validate_tests(result,config['native_tests']);gates['native_material2']='passed'
    finally:
        assert_seal(ROOT,before_mod,True)
        for path,before in snapshots.items():assert_seal(Path(path),before)
        if stage_before is not None:assert_seal(stage,stage_before)
        if generated is not None and out_dirs(work/'host-target')!=generated:raise ValueError('Generated native inputs changed during material tests')
        report={'head':os.environ['GITHUB_SHA'],'production_source':config['remote_source'],'bundle_sha256':config['bundle_sha256'],
                'gates':gates,'minimum_free_bytes':runner.minimum_free,'floor_bytes':FLOOR,
                'scope':'Plan/apply/type and pure/material contracts only; no consumer activation, live loader, meshing differential, timing, gameplay or GPU proof',
                'stage_seal_sha256':sha(json.dumps(stage_before,sort_keys=True).encode()) if stage_before else None}
        print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:f.write('## Fluid render bounded proof\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')

if __name__=='__main__':
    if len(sys.argv)==5 and sys.argv[1]=='--export':export(Path(sys.argv[2]),sys.argv[3],Path(sys.argv[4]))
    elif len(sys.argv)==4 and sys.argv[1]=='--unpack':unpack(Path(sys.argv[2]),Path(sys.argv[3]))
    elif len(sys.argv)==1:main()
    else:raise SystemExit('Invalid source runner arguments')
