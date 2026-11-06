#!/usr/bin/env python3
"""Single complete Pluto composition: host typing and 18 scoped native tests."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import shlex
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

def ensure_group_gone(pgid, name):
    deadline = time.monotonic() + 1.0
    while True:
        try: os.killpg(pgid, 0)
        except ProcessLookupError: break
        if time.monotonic() >= deadline: raise RuntimeError('Own process group remains: ' + name)
        time.sleep(0.02)

class Runner:
    def __init__(self, work, env, report=None):
        self.work, self.env = work, env
        self.minimum_free = None
        self.report = report if report is not None else {"phases": []}
    def guard(self, reserve=0):
        free = min(shutil.disk_usage(p).free for p in write_filesystems(self.work, self.env))
        self.minimum_free = free if self.minimum_free is None else min(self.minimum_free, free)
        if free < FLOOR + reserve: raise RuntimeError('Disk floor reached; own process group stopped')
    def run(self, name, command, cwd=None, timeout=2400):
        self.guard()
        path = self.work / (name + '.log')
        print('PHASE_START', name, json.dumps(command), flush=True)
        started = time.monotonic()
        proc = None
        error = None
        try:
            with path.open('w') as log:
                proc = subprocess.Popen(command, cwd=cwd or ROOT, env=self.env, stdout=log,
                                        stderr=subprocess.STDOUT, start_new_session=True)
                try:
                    while proc.poll() is None:
                        self.guard()
                        if time.monotonic() - started > timeout: raise TimeoutError(name)
                        time.sleep(0.2)
                finally:
                    if proc.poll() is None:
                        try: os.killpg(proc.pid, signal.SIGTERM)
                        except ProcessLookupError: pass
                        deadline = time.monotonic() + 1.0
                        while proc.poll() is None and time.monotonic() < deadline: time.sleep(0.05)
                    try: os.killpg(proc.pid, signal.SIGKILL)
                    except ProcessLookupError: pass
                    proc.wait(timeout=2)
                    ensure_group_gone(proc.pid, name)
            self.guard()
            if proc.returncode != 0: raise RuntimeError(name + ' exited ' + str(proc.returncode))
            return path.read_text()
        except BaseException as exc:
            error = exc
            raise
        finally:
            raw = path.read_bytes() if path.exists() else b''
            text = raw.decode(errors='replace')
            phase = {'name':name, 'seconds':round(time.monotonic()-started,3),
                     'exit_code':proc.returncode if proc is not None else None,
                     'raw_sha256':sha(raw), 'raw_bytes':len(raw),
                     'error':None if error is None else type(error).__name__+': '+str(error)}
            self.report['phases'].append(phase)
            try:
                phase['compilers'] = compiler_records(text)
                print_compact_log(text, error is not None)
            except BaseException as exc:
                phase['summary_error'] = type(exc).__name__+': '+str(exc)
                raise
            finally:
                print('PHASE_RECEIPT',json.dumps(phase,sort_keys=True),flush=True)

COMPILER_TARGETS={'pluto_api','pluto_inventory_api','horizon_client_render','horizon_client_lib','horizon_client'}
def compiler_records(log):
    records=[]
    for line in log.splitlines():
        if 'Running `' not in line or 'rustc' not in line: continue
        args=shlex.split(line.split('Running `',1)[1].rsplit('`',1)[0])
        if '--crate-name' not in args: continue
        name=args[args.index('--crate-name')+1]
        if name not in COMPILER_TARGETS: continue
        records.append({'crate':name,'source':[a for a in args if a.endswith('.rs')],
                        'test':'--test' in args,'invocation_sha256':sha(line.encode())})
    return records

def print_compact_log(log, failed):
    selected='\n'.join(line for line in log.splitlines()
        if line.startswith(('test ','test result:','Ran ','OK','Native plan','Pluto')))
    if len(selected.encode())>128*1024: raise ValueError('Selected console evidence exceeds bound')
    if selected: print(selected,flush=True)
    if failed:
        errors=[]
        for line in log.splitlines():
            if not line.startswith('{'):continue
            try:value=json.loads(line)
            except json.JSONDecodeError:continue
            message=value.get('message',{})
            if value.get('reason')=='compiler-message' and message.get('level')=='error':
                errors.append(str(message.get('rendered') or message.get('message',''))[:4096])
        print('FAILURE_DIAGNOSTICS','\n'.join(errors)[:16384],flush=True)
        print('FAILURE_TAIL',log[-16384:],flush=True)


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

def native_test_compilation(log, name, source):
    native_rustc(log,[name])
    if not any('Running `' in line and 'rustc' in line and '--crate-name '+name+' ' in line and
               ' --test ' in line and source in line for line in log.splitlines()):
        raise ValueError('Missing actual native unit-test compilation: '+name)

def native_host_checks(log):
    native_rustc(log,['pluto_api','pluto_inventory_api','horizon_client_render','horizon_client_lib','horizon_client'])
    for path in ['src/lib.rs','src/main.rs']:
        if not any('Running `' in line and 'rustc' in line and '--crate-name horizon_client ' in line and path in line for line in log.splitlines()):
            raise ValueError('Missing actual host target check: '+path)

def validate_python_checks(log, count):
    summaries=re.findall(r'^Ran (\d+) tests? in .+$',log,re.M)
    successes=re.findall(r'^test_\S+ .* \.\.\. ok$',log,re.M)
    if summaries!=[str(count)] or len(successes)!=count or re.findall(r'^OK$',log,re.M)!=['OK']:
        raise ValueError('Expected exactly '+str(count)+' executed passing Python checks')

def record_error(report, error, prefix=''):
    value=prefix+type(error).__name__+': '+str(error)
    report['error']=(report['error']+'; ' if report['error'] else '')+value

def checkout_seal(root):
    """Seal ignored/untracked entries too, without following source symlinks."""
    import stat
    result={}
    for p in sorted(root.rglob('*')):
        relative=p.relative_to(root)
        if '.git' in relative.parts:continue
        mode=p.lstat().st_mode
        if stat.S_ISLNK(mode):value={'kind':'symlink','target':os.readlink(p)}
        elif stat.S_ISDIR(mode):value={'kind':'directory'}
        elif stat.S_ISREG(mode):value={'kind':'file','sha256':sha(p.read_bytes()),'mode':stat.S_IMODE(mode)}
        else:raise ValueError('Unexpected special source entry: '+str(p))
        result[relative.as_posix()]=value
    return result

def assert_full_seal(root, expected):
    if checkout_seal(root)!=expected:raise ValueError('Full source/mode/link/directory drift: '+str(root))

def assert_git_clean(root):
    if git(root,'status','--porcelain','--untracked-files=all','--ignored=matching'):
        raise ValueError('Dirty/untracked/ignored checkout: '+str(root))

def assert_clean_checkout():
    assert_git_clean(ROOT)


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


RUNNER_TEST_COUNT=23
REQUIRED_GATES={'runner','language_source19','terrain_source23','composition_source4','package','plan','complete_plan_guards','apply','host_type','client_lib_compile','renderer_compile','test_inventory','internal8','key6','facade2','material2'}

def validate_inputs(config):
    expected_official={'horizon':{'url':'https://github.com/horizon-rust/horizon.git','revision':'624c4f6265c02296fbe8a80f029dd0d987fb94c0'},'mixins':{'url':'https://github.com/horizon-rust/horizon-mixins.git','revision':'e8cd0aa38cffef49cb2e088ef716baa107148c02'}}
    if config['official']!=expected_official:raise ValueError('Require the exact official repositories and revisions')
    if (config['repository']!='sofia-gros/horizon-mods' or
        config['branch']!='checkpoint/plutolib-common-composite-ci-20261106' or
        config['runtime_source']!='035273c4805d75111e528c8c26607fa3a8d1d5bf' or
        config['bundle_sha256']!='4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d' or
        config['official_lock_sha256']!='463913123985e344f2889e5beae65982773e6dbfacdcac7a916bcec522a5bd31' or
        config['manifest_counts']!={'crate':2,'module':69,'injection':34,'state':3}):
        raise ValueError('Require exact common source, archive, lock and composition')
    for path,digest in config['source_sha256'].items():
        if sha((ROOT/path).read_bytes())!=digest:raise ValueError('Frozen source changed: '+path)
    expected={'key6':('client_lib','pluto_language_text::tests::',6,4),
              'internal8':('client_lib','pluto_language::tests::',8,4),
              'facade2':('client_lib','pluto_render::terrain::tests::',2,8),
              'material2':('renderer','mesher::pluto_terrain_cell_host::material_contract_tests::',2,4)}
    if len(config['groups'])!=4 or {g['gate'] for g in config['groups']}!=set(expected):
        raise ValueError('Missing or duplicate scoped group')
    for group in config['groups']:
        if (group['target'],group['prefix'],group['count'],group['indent'])!=expected[group['gate']]:
            raise ValueError('Unexpected target, selector, count or source shape')
        source=(ROOT/group['path']).read_text()
        if group['marker']:
            if source.count(group['marker'])!=1:raise ValueError('Invalid source marker')
            source=source.split(group['marker'],1)[1]
        pad=' '*group['indent']
        names=[group['prefix']+n for n in re.findall(r'^'+pad+r'#\[test\]\n'+pad+r'fn (\w+)\(',source,re.M)]
        if len(names)!=group['count'] or len(set(group['names']))!=group['count'] or set(names)!=set(group['names']):
            raise ValueError('Installed source test inventory differs')
    if config['targets']!={'client_lib':{'package':'horizon-client-lib','crate':'horizon_client_lib','source':'crates/horizon-client/src/client_lib.rs'},'renderer':{'package':'horizon-client-render','crate':'horizon_client_render','source':'crates/horizon-client-render/src/lib.rs'}}:
        raise ValueError('Changed native test owner')

def verify_plan(plan,config):
    value=json.loads(plan.read_text()); found={}
    for row in value.get('modules',[]):
        name=row.get('name')
        if name in config['installed_modules']:
            if name in found:raise ValueError('Duplicate installed composite module')
            found[name]=row
    if set(found)!=set(config['installed_modules']):raise ValueError('Missing installed composite modules')
    for name,expected in config['installed_modules'].items():
        row=found[name]
        for field in ('file','visibility','mod_id'):
            if row.get(field)!=expected[field]:raise ValueError('Wrong native module owner: '+name)
        if row.get('source')!=(ROOT/expected['path']).read_text():raise ValueError('Planned module payload changed: '+name)
    return {name:sha(row['source'].encode()) for name,row in found.items()}

def select_test_artifact(log, target, stage, output):
    found=[]
    for line in log.splitlines():
        if not line.startswith('{'):continue
        try:row=json.loads(line)
        except json.JSONDecodeError:continue
        if row.get('reason')!='compiler-artifact':continue
        rust_target=row.get('target',{});profile=row.get('profile',{})
        if rust_target.get('name')!=target['crate'] or profile.get('test') is not True:continue
        if rust_target.get('kind')!=['lib'] or Path(rust_target.get('src_path','')).resolve()!=(stage/target['source']).resolve():
            raise ValueError('Wrong actual native test artifact source')
        if profile.get('opt_level')!='0' or profile.get('debug_assertions') is not True:
            raise ValueError('Wrong test profile')
        binary=Path(row.get('executable') or '').resolve()
        if not binary.is_relative_to((output/'debug/deps').resolve()) or not binary.is_file() or not os.access(binary,os.X_OK):
            raise ValueError('Executable is outside isolated Cargo target')
        found.append(binary)
    if len(found)!=1:raise ValueError('Missing or duplicate native test artifact')
    return found[0]

def validate_list(log, groups):
    names=re.findall(r'^(\S+): test$',log,re.M)
    summaries=re.findall(r'^(\d+) tests, 0 benchmarks$',log,re.M)
    if len(names)!=len(set(names)) or summaries!=[str(len(names))] or not names:
        raise ValueError('Unexpected native test inventory summary')
    for group in groups:
        selected=[n for n in names if group['prefix'] in n]
        if len(selected)!=group['count'] or set(selected)!=set(group['names']):
            raise ValueError('Actual libtest selector includes missing/extra tests')
    return len(names)

def assert_binaries(binaries):
    for path,digest in binaries.values():
        if not path.is_file() or sha(path.read_bytes())!=digest:raise ValueError('Compiled test binary drift')


def execute(report):
    config=read_inputs();event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if (os.environ.get('GITHUB_ACTIONS')!='true' or os.environ.get('GITHUB_REPOSITORY')!=config['repository'] or
        os.environ.get('GITHUB_EVENT_NAME')!='push' or os.environ.get('GITHUB_REF')!='refs/heads/'+config['branch'] or
        event.get('repository',{}).get('private') is not False or git(ROOT,'rev-parse','HEAD')!=os.environ.get('GITHUB_SHA') or
        git(ROOT,'status','--porcelain','--untracked-files=all','--ignored=matching')):raise ValueError('Require exact clean authorized public CI branch')
    validate_inputs(config)
    original=seal(ROOT,True);whole_original=checkout_seal(ROOT)
    work=Path(os.environ['RUNNER_TEMP'])/('common-composite-'+os.environ['GITHUB_SHA'])
    env=os.environ.copy()
    for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:env.pop(key,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'host-target'),
        CARGO_BUILD_JOBS='1',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_TEST_OPT_LEVEL='0',CARGO_PROFILE_RELEASE_DEBUG='0',
        CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1')
    runner=Runner(work,env,report);runner.guard(reserve=2*1024**3)
    work.mkdir();Path(env['CARGO_HOME']).mkdir();Path(env['TMPDIR']).mkdir()
    gates=report['gates'];snapshots={};checkouts={};stage=None;stage_seal=None;generated=None;binaries={}
    try:
        checked=runner.run('runner-checks',[sys.executable,str(HERE/'test_runner.py'),'-v'])
        validate_python_checks(checked,RUNNER_TEST_COUNT);gates['runner']={'passed':True,'tests':RUNNER_TEST_COUNT}
        for gate,pattern,count in [('language_source19','test_language*.py',19),('terrain_source23','test_terrain*.py',23),('composition_source4','test_common_composite.py',4)]:
            source=runner.run(gate,[sys.executable,'-m','unittest','discover','-s','pluto-lib/tests','-p',pattern,'-v'])
            validate_python_checks(source,count);gates[gate]={'passed':True,'tests':count}
        runner.run('apt-update',['sudo','apt-get','update'],timeout=600)
        runner.run('apt-install',['sudo','apt-get','install','-y','--no-install-recommends','libasound2-dev','libudev-dev'],timeout=600)
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal','--component','rustfmt'],timeout=600)
        for name,info in config['official'].items():
            checkout=work/(name+'-git');original_tree=work/name
            runner.run(name+'-init',['git','init',str(checkout)])
            runner.run(name+'-origin',['git','-C',str(checkout),'remote','add','origin',info['url']])
            runner.run(name+'-fetch',['git','-C',str(checkout),'fetch','--depth','1','origin',info['revision']])
            runner.run(name+'-checkout',['git','-C',str(checkout),'checkout','--detach','FETCH_HEAD'])
            if git(checkout,'rev-parse','HEAD')!=info['revision'] or git(checkout,'remote','get-url','origin')!=info['url'] or git(checkout,'status','--porcelain','--untracked-files=all','--ignored=matching'):
                raise ValueError('Official provenance or cleanliness changed')
            checkouts[str(checkout)]=checkout_seal(checkout)
            runner.run(name+'-export',[sys.executable,str(HERE/'run.py'),'--export',str(checkout),info['revision'],str(original_tree)])
            snapshots[str(original_tree)]=checkout_seal(original_tree)
        official=work/'horizon';mixins=work/'mixins'
        if sha((official/'Cargo.lock').read_bytes())!=config['official_lock_sha256']:raise ValueError('Wrong official lock')
        bundle=work/'plutolib.hnmod';mods=work/'package'
        runner.run('package',[sys.executable,str(ROOT/'pluto-lib/tools/package_native.py'),str(bundle)])
        if sha(bundle.read_bytes())!=config['bundle_sha256']:raise ValueError('Wrong complete native bundle')
        runner.run('unpack',[sys.executable,str(HERE/'run.py'),'--unpack',str(bundle),str(mods)])
        import tomllib
        manifest=tomllib.loads((mods/'mixins/pluto_native_creative/horizon-mixin.toml').read_text())
        counts={k:len(manifest.get(k,[])) for k in ['crate','module','injection','state']}
        if counts!=config['manifest_counts']:raise ValueError('Unexpected whole package composition')
        snapshots[str(mods)]=checkout_seal(mods);gates['package']={'passed':True,'counts':counts,'sha256':config['bundle_sha256']}
        runner.run('mixins-build',['cargo','build','--release','--locked','-p','horizon-mixins-cli','--target-dir',str(work/'mixins-target')],cwd=mixins)
        cli=work/'mixins-target/release/horizon-mixins';plan=work/'plan.json';stage=work/'stage'
        runner.run('official-plan',[str(cli),'plan','--workspace',str(official),'--mods',str(mods/'mixins'),'--out',str(plan)])
        module_seals=verify_plan(plan,config);gates['plan']={'passed':True,'sha256':sha(plan.read_bytes()),'public_modules':module_seals}
        runner.run('complete-plan-guards',[sys.executable,str(ROOT/'pluto-lib/tools/check_native_plan.py'),str(plan),str(official)])
        gates['complete_plan_guards']={'passed':True,'required_groups':12}
        runner.run('official-apply',[str(cli),'apply','--workspace',str(official),'--mods',str(mods/'mixins'),'--out',str(stage)])
        stage_seal=checkout_seal(stage);gates['apply']={'passed':True}
        runner.run('stage-fetch',['cargo','fetch','--locked','--manifest-path',str(stage/'Cargo.toml')])
        typed=runner.run('host-type',['cargo','check','--offline','--locked','--manifest-path',str(stage/'Cargo.toml'),'-p','horizon-client','--no-default-features','--lib','--bin','horizon-client','-vv'],cwd=stage)
        native_host_checks(typed);assert_full_seal(stage,stage_seal);gates['host_type']={'passed':True,'targets':['horizon_client_lib','horizon_client lib','horizon_client bin']}
        for name,target in config['targets'].items():
            command=['cargo','test','--offline','--locked','--manifest-path',str(stage/'Cargo.toml'),'-p',target['package'],'--no-default-features','--lib','--no-run','--message-format=json-render-diagnostics','-vv']
            built=runner.run(name+'-compile',command,cwd=stage)
            native_test_compilation(built,target['crate'],Path(target['source']).name)
            binary=select_test_artifact(built,target,stage,work/'host-target')
            binaries[name]=(binary,sha(binary.read_bytes()))
            gates[name+'_compile']={'passed':True,'source':target['source'],'binary_sha256':binaries[name][1]}
            assert_full_seal(stage,stage_seal)
        # Both native compilations finish before capturing generated inputs.
        generated=out_dirs(work/'host-target');totals={}
        for name,(binary,digest) in binaries.items():
            assert_binaries(binaries)
            listing=runner.run(name+'-list',[str(binary),'--list'],cwd=stage)
            totals[name]=validate_list(listing,[g for g in config['groups'] if g['target']==name])
        gates['test_inventory']={'passed':True,'total_tests_by_target':totals}
        for group in config['groups']:
            assert_binaries(binaries);assert_full_seal(stage,stage_seal)
            if out_dirs(work/'host-target')!=generated:raise ValueError('Generated inputs drift before execution')
            binary=binaries[group['target']][0]
            result=runner.run(group['gate'],[str(binary),group['prefix'],'--test-threads=1'],cwd=stage)
            # Direct sealed libtest executables: no Cargo or rustc execution path.
            if any('Running `' in line and 'rustc' in line for line in result.splitlines()):raise ValueError('Recompilation in execution phase')
            validate_tests(result,group['names'],totals[group['target']]-group['count'])
            assert_binaries(binaries);assert_full_seal(stage,stage_seal)
            if out_dirs(work/'host-target')!=generated:raise ValueError('Generated inputs drift during execution')
            gates[group['gate']]={'passed':True,'tests':group['count'],'names':group['names']}
    except BaseException as exc:record_error(report,exc)
    finally:
        try:
            assert_seal(ROOT,original,True);assert_clean_checkout()
            if checkout_seal(ROOT)!=whole_original:raise ValueError('Full Mod checkout drift')
            for path,before in checkouts.items():
                assert_git_clean(Path(path))
                if checkout_seal(Path(path))!=before:raise ValueError('Full official checkout drift')
            for path,before in snapshots.items():assert_full_seal(Path(path),before)
            if stage_seal is not None:assert_full_seal(stage,stage_seal)
            if generated is not None and out_dirs(work/'host-target')!=generated:raise ValueError('Generated source drift')
            assert_binaries(binaries)
            runner.guard();report['verified']=True
        except BaseException as exc:record_error(report,exc,'verification: ')
        report.update(minimum_free_bytes=runner.minimum_free,source_seal_sha256=sha(json.dumps(original,sort_keys=True).encode()),
            full_checkout_seal_sha256=sha(json.dumps(whole_original,sort_keys=True).encode()),
            official_checkout_seals={path:sha(json.dumps(value,sort_keys=True).encode()) for path,value in checkouts.items()},
            stage_seal_sha256=sha(json.dumps(stage_seal,sort_keys=True).encode()) if stage_seal else None,
            generated_output_seal_sha256=sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None)

def main():
    report={'head':os.environ.get('GITHUB_SHA'),'passed':False,'verified':False,'error':None,'gates':{},'phases':[],'floor_bytes':FLOOR,
        'scope':'Single exact common Pluto bundle: plan/12guards/apply, actual host lib+bin typing and installed language14, terrain facade2, renderer material2. No live Game, GUI, GPU, LAN, reload, pack/content overlay, full TConstruct or pure-API suite rerun claim.'}
    try:
        execute(report)
        if report['error'] is None and set(report['gates'])!=REQUIRED_GATES:raise ValueError('Missing required gate')
    except BaseException as exc:record_error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:f.write('## Common Pluto composition\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:record_error(report,exc,'summary: ');report['passed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)

if __name__=='__main__':
    if len(sys.argv)==5 and sys.argv[1]=='--export':export(Path(sys.argv[2]),sys.argv[3],Path(sys.argv[4]))
    elif len(sys.argv)==4 and sys.argv[1]=='--unpack':unpack(Path(sys.argv[2]),Path(sys.argv[3]))
    elif len(sys.argv)==1:main()
    else:raise SystemExit('Invalid runner arguments')
