#!/usr/bin/env python3
"""Exact official mod_content::build baseline; no merge repair, Game or UI claim."""
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
    # Seal all files (including ignored/untracked names) and symlink targets.
    # The official checkout contains a tracked documentation symlink, so retain
    # its exact link rather than rejecting all links or following external trees.
    import stat
    result={}
    for path in sorted(root.rglob('*')):
        rel=path.relative_to(root)
        if '.git' in rel.parts:continue
        mode=path.lstat().st_mode
        if stat.S_ISDIR(mode):continue
        if stat.S_ISREG(mode):result[str(rel)]={'kind':'file','sha256':sha(path.read_bytes())}
        elif stat.S_ISLNK(mode):result[str(rel)]={'kind':'symlink','target':os.readlink(path)}
        else:raise ValueError('Special checkout source: '+str(path))
    return result

def assert_seal(root, expected, tracked=False):
    if seal(root) != expected: raise ValueError('Source/lock/file-set drift: ' + str(root))
def out_dirs(target):
    return {str(p.relative_to(target)): sha(p.read_bytes())
            for out in sorted(target.glob('debug/build/*/out')) for p in sorted(out.rglob('*')) if p.is_file()}


def emit_log_summary(name,path,returncode):
    # Validators consume the original complete log, not this bounded console view.
    data=path.read_bytes()
    print('RAW_LOG',json.dumps({'phase':name,'bytes':len(data),'sha256':sha(data),'exit':returncode}),flush=True)
    if returncode != 0:
        print('FAILURE_TAIL',data[-16000:].decode('utf-8',errors='replace'),flush=True)

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
                emit_log_summary(name,path,proc.returncode)
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
            if not test or ' --test' in line:
                print('ACTUAL_COMPILE_WITNESS',json.dumps({'crate':crate,'test':test,'line_sha256':sha(line.encode())}),flush=True)
                return
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
    names=validate_suite(output,count,required)
    print('VALIDATED_TEST_NAMES',json.dumps({'phase':name,'names':names,'passed':count,'ignored':0,'filtered':0}),flush=True)
    return names

def validate_python_checks(log, count):
    if (re.findall(r'^Ran (\d+) tests? in .+$',log,re.M)!=[str(count)] or
        len(re.findall(r'^test_\S+ .* \.\.\. ok$',log,re.M))!=count or
        re.findall(r'^OK$',log,re.M)!=['OK']):
        raise ValueError('Expected exactly '+str(count)+' unskipped Python checks')

def report_error(report,error,prefix=''):
    msg=prefix+type(error).__name__+': '+str(error)
    report['error']=(report['error']+'; ' if report['error'] else '')+msg

REQUIRED_GATES={'runner','source','metadata','actual_compile','tests::'}
OFFICIAL={'url':'https://github.com/horizon-rust/horizon.git',
          'revision':'624c4f6265c02296fbe8a80f029dd0d987fb94c0',
          'lock_sha256':'463913123985e344f2889e5beae65982773e6dbfacdcac7a916bcec522a5bd31'}

REPOSITORY='sofia-gros/horizon-mods'
BRANCH='checkpoint/plutolib-language-overlay-ci-20261106'
BASE='783deeb11bbe63e72bbaefc1954439c3219d4f9a'

def validate_official(config):
    if (config['repository'],config['branch'],config['base'])!=(REPOSITORY,BRANCH,BASE):
        raise ValueError('Wrong fixed repository, trigger branch or source base')
    if config['official']!=OFFICIAL:raise ValueError('Only the exact approved official source is allowed')

def execute(report):
    config=read_inputs();validate_official(config);report['cutoffs']=config['cutoffs']
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if (os.environ.get('GITHUB_ACTIONS')!='true' or os.environ.get('GITHUB_REPOSITORY')!=config['repository'] or
        os.environ.get('GITHUB_EVENT_NAME')!='push' or os.environ.get('GITHUB_REF')!='refs/heads/'+config['branch'] or
        event.get('repository',{}).get('private') is not False or git(ROOT,'rev-parse','HEAD')!=os.environ['GITHUB_SHA'] or
        git(ROOT,'status','--porcelain','--untracked-files=all','--ignored=matching')):
        raise ValueError('Require exact clean authorized public CI branch')
    for p,digest in config['inputs_sha256'].items():
        if sha((ROOT/p).read_bytes())!=digest:raise ValueError('Frozen input mismatch: '+p)
    expected=[name for g in config['result_groups'] for name in g['names']]
    if len(expected)!=8 or len(set(expected))!=8 or [g['expected'] for g in config['result_groups']]!=[8]:
        raise ValueError('Expected the one complete eight-case overlay baseline group')
    original=seal(ROOT,True)
    work=Path(os.environ['RUNNER_TEMP'])/('pluto-language-overlay-'+os.environ['GITHUB_SHA']);env=os.environ.copy()
    for k in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:env.pop(k,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'target'),
               CARGO_BUILD_JOBS='1',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',
               CARGO_TERM_COLOR='never',TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1')
    if min(shutil.disk_usage(p).free for p in write_filesystems(work,env))<FLOOR+2*1024**3:
        raise RuntimeError('Require floor plus 2 GiB setup reserve')
    work.mkdir();Path(env['CARGO_HOME']).mkdir();Path(env['TMPDIR']).mkdir()
    runner=Runner(work,env);gates=report['gates'];official=None;official_seal=None;fixture=None;fixture_seal=None;generated=None
    try:
        output=runner.run('runner-checks',[sys.executable,str(HERE/'test_runner.py'),'-v'])
        validate_python_checks(output,config['runner_tests']);gates['runner']={'passed':True,'tests':config['runner_tests']}
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal'],timeout=600)
        official=work/'official';pin=config['official']
        runner.run('official-init',['git','init',str(official)])
        runner.run('official-origin',['git','-C',str(official),'remote','add','origin',pin['url']])
        runner.run('official-fetch',['git','-C',str(official),'fetch','--depth','1','origin',pin['revision']])
        runner.run('official-checkout',['git','-C',str(official),'checkout','--detach','FETCH_HEAD'])
        if git(official,'rev-parse','HEAD')!=pin['revision'] or git(official,'remote','get-url','origin')!=pin['url'] or git(official,'status','--porcelain','--untracked-files=all','--ignored=matching'):
            raise ValueError('Wrong or dirty official source')
        if sha((official/'Cargo.lock').read_bytes())!=pin['lock_sha256']:raise ValueError('Wrong official lock')
        official_seal=seal(official,True)
        source_driver=("import importlib.util,sys,unittest; from pathlib import Path; "
            "spec=importlib.util.spec_from_file_location('overlay_source',sys.argv[1]); "
            "m=importlib.util.module_from_spec(spec); spec.loader.exec_module(m); "
            "m.HOST=Path(sys.argv[2]); "
            "r=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromModule(m)); "
            "sys.exit(not r.wasSuccessful())")
        output=runner.run('source-checks',[sys.executable,'-c',source_driver,
            str(ROOT/'pluto-lib/experiments/language-overlay/test_source.py'),str(official)])
        validate_python_checks(output,config['python_tests']);gates['source']={'passed':True,'tests':config['python_tests']}
        runner.run('official-locked-fetch',['cargo','fetch','--locked','--manifest-path',str(official/'Cargo.toml')])
        fixture=work/'fixture';preparer=HERE/'prepare.py'
        runner.run('prepare',[sys.executable,str(preparer),'--official',str(official),'--output',str(fixture)])
        runner.run('metadata-resolve',[sys.executable,str(preparer),'--official',str(official),'--output',str(fixture),'--resolve'])
        runner.run('verify-before',[sys.executable,str(preparer),'--official',str(official),'--output',str(fixture),'--verify'])
        evidence=json.loads((fixture/'fixture-sources.json').read_text())
        if evidence['revision']!=pin['revision'] or evidence['expected_names']!=expected:
            raise ValueError('Fixture evidence differs from exact CI scope')
        fixture_seal=regular_seal(fixture);gates['metadata']={'passed':True,'packages':evidence['dependency_packages']}
        cmd=['cargo','test','--lib','--locked','--offline','--manifest-path',str(fixture/'Cargo.toml'),'-vv']
        built=runner.run('actual-native-compile',cmd+['--no-run'],timeout=1800)
        for crate in ['horizon_client_core','horizon_mods','horizon_client_lib']:
            actual_compile(built,crate)
        actual_compile(built,'pluto_language_overlay_fixture',True)
        gates['actual_compile']={'passed':True,'native_crates':['horizon_client_core','horizon_mods','horizon_client_lib'],'fixture':'actual public mod_content::build and PathBuf ResourceSource; owned minimal fixture roots; no Game admission, priority repair or live UI'}
        generated=out_dirs(work/'target')
        names=test_after_compile(runner,'native8',cmd+['--','--test-threads=1'],8,expected)
        for group in config['result_groups']:
            got=[n for n in names if n in group['names']]
            if len(got)!=group['expected']:raise ValueError('Missing native group: '+group['prefix'])
            gates[group['prefix']]={'passed':True,'tests':len(got),'names':got}
        runner.run('verify-after',[sys.executable,str(preparer),'--official',str(official),'--output',str(fixture),'--verify'])
        if set(gates)!=REQUIRED_GATES:
            raise ValueError('Missing required gate')
    except BaseException as exc:report_error(report,exc)
    finally:
        try:
            assert_seal(ROOT,original,True)
            if git(ROOT,'status','--porcelain','--untracked-files=all','--ignored=matching'):raise ValueError('Dirty/untracked source checkout')
            if official_seal is not None:
                assert_seal(official,official_seal,True)
                if git(official,'status','--porcelain','--untracked-files=all','--ignored=matching'):raise ValueError('Dirty/untracked official checkout')
            if fixture_seal is not None and regular_seal(fixture)!=fixture_seal:raise ValueError('Fixture source/lock/file-set drift')
            if generated is not None and out_dirs(work/'target')!=generated:raise ValueError('Generated input drift')
            runner.guard();report['verified']=True
        except BaseException as exc:report_error(report,exc,'verification: ')
        report['minimum_free_bytes']=runner.minimum_free
        report['source_seal_sha256']=sha(json.dumps(original,sort_keys=True).encode())
        report['official_seal_sha256']=sha(json.dumps(official_seal,sort_keys=True).encode()) if official_seal else None
        report['fixture_seal_sha256']=sha(json.dumps(fixture_seal,sort_keys=True).encode()) if fixture_seal else None
        report['generated_output_seal_sha256']=sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None

def main():
    report={'head':os.environ.get('GITHUB_SHA'),'passed':False,'verified':False,'error':None,'gates':{},'floor_bytes':FLOOR,
            'scope':'Actual official mod_content::build, Package/Content and native Language on eight owned minimal baseline cases. Existing whole-file Mod overlay only. No installed Game admission, selected/server pack merge or filters, priority repair, Options/live UI, rendering, LAN or runtime TOML claim.'}
    try:
        execute(report)
        if report['error'] is None and set(report['gates'])!=REQUIRED_GATES:raise ValueError('Missing required gate')
    except BaseException as exc:report_error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:f.write('## Native language overlay baseline contracts\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:report_error(report,exc,'summary: ');report['passed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)

if __name__=='__main__':main()
