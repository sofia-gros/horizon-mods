#!/usr/bin/env python3
"""Excluded item-name candidate: actual applied regions and codec32, not Game."""
import hashlib,json,os,re,shlex,shutil,signal,subprocess,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
HERE=Path(__file__).resolve().parent
BASE=HERE.parent
FLOOR=10*1024**3+128*1024**2
COMPILER_TARGETS={'horizon_core','horizon_client_core','horizon_items','horizon_protocol','horizon_world','pluto_item_name_removal_tests','horizon_mixins'}
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

REPOSITORY='sofia-gros/horizon-mods'
BRANCH='checkpoint/plutolib-item-name-removal-ci-20261106'
SOURCE_BASE='47422d690b08f9db0575e5b5cb5d2260cb04f02f'
OFFICIAL={
 'horizon':{'url':'https://github.com/horizon-rust/horizon.git','revision':'624c4f6265c02296fbe8a80f029dd0d987fb94c0','lock_sha256':'463913123985e344f2889e5beae65982773e6dbfacdcac7a916bcec522a5bd31'},
 'mixins':{'url':'https://github.com/horizon-rust/horizon-mixins.git','revision':'e8cd0aa38cffef49cb2e088ef716baa107148c02','lock_sha256':'35846f4a621cf8e1e54f6f0f7e13ac848db590d4ec401620501c107954eae6ca'},
}
REQUIRED_GATES={'runner','guard_checks','preparer_checks','official_sources','mixins_compile',
 'plan','exclusive_guard','apply','applied_regions','metadata','native_compile','test_inventory',
 'native_codec','entry_models','owned_tag'}

INPUT_PREFIX='pluto-lib/experiments/item-name-connection/'
REQUIRED_INPUTS={INPUT_PREFIX+p for p in [
 'candidate/README.md','candidate/entry-gate.rs','candidate/horizon-mixin.toml',
 'candidate/name-removal.rs','candidate/receive.rs','candidate/send.rs',
 'validation/README.md','validation/inputs.json','validation/check_plan.py','validation/test_plan.py',
 'validation/codec_tests.rs','validation/prepare_fixture.py','validation/fixture-inputs.json','validation/test_prepare.py',
 'ci/run.py','ci/test_runner.py','ci/README.md']}
REQUIRED_INPUTS|={'tconstruct/tools/portable_archive.py','.github/workflows/item-name-removal-wip.yml'}

def validate_inputs(config):
    if (config['repository'],config['branch'],config['base'])!=(REPOSITORY,BRANCH,SOURCE_BASE):
        raise ValueError('Wrong repository, dedicated trigger branch or source base')
    if config['official']!=OFFICIAL: raise ValueError('Only the fixed official sources are allowed')
    if config['runtime_source']!='0f528de0332afc5d86910dd07b08dcebe98a04dd':
        raise ValueError('Wrong isolated runtime source')
    if set(config['source_sha256'])!=REQUIRED_INPUTS:raise ValueError('Frozen input file-set differs')
    for path,digest in config['source_sha256'].items():
        p=ROOT/path
        if p.is_symlink() or sha(p.read_bytes())!=digest:raise ValueError('Frozen input mismatch: '+path)
    expected=json.loads((BASE/'validation/fixture-inputs.json').read_text())
    if expected['expected_tests']!=32 or config['groups']!=expected['result_groups']:
        raise ValueError('Expected the complete source-derived 32-test inventory')
    names=[]
    for group in config['groups']:
        body=(BASE/'validation'/group['source']).read_text()
        actual=[group['prefix']+n for n in re.findall(r'#\[test\]\s*fn ([a-z0-9_]+)',body)]
        if actual!=group['tests']:raise ValueError('Test source names changed')
        names+=actual
    if len(names)!=32 or len(set(names))!=32:raise ValueError('Duplicate/missing native test names')

def assert_event(config,event,env):
    if (env.get('GITHUB_ACTIONS')!='true' or env.get('GITHUB_REPOSITORY')!=REPOSITORY or
        env.get('GITHUB_EVENT_NAME')!='push' or env.get('GITHUB_REF')!='refs/heads/'+BRANCH or
        event.get('repository',{}).get('private') is not False):
        raise ValueError('Require exact public dedicated push branch')

def output_seal(target):
    result={}
    for out in sorted(target.glob('debug/build/*/out')):
        tree=checkout_seal(out)
        if any(row['kind']=='symlink' for row in tree.values()):raise ValueError('Symlink in generated source')
        result[out.relative_to(target).as_posix()]=tree
    return result

def validate_cli(log):
    value=json.loads(log)
    expected={'injections':3,'modules':2,'bridges':0,'states':0,'crates':0,'data':0,'post_build':0}
    if value.get('status')!='ok' or any(value.get(k)!=v for k,v in expected.items()):
        raise ValueError('Unexpected candidate plan/apply composition')
    return expected

def require_compile(log,crate,test=False):
    selected=[r for r in compiler_records(log) if r['crate']==crate and r['test']==test]
    if not selected:raise ValueError('Missing actual compiler invocation: '+crate)
    if test and len(selected)!=1:raise ValueError('Expected exactly one fixture test compilation')
    return selected

def select_artifact(log,fixture,target):
    found=[]
    for line in log.splitlines():
        if not line.startswith('{'):continue
        try:row=json.loads(line)
        except json.JSONDecodeError:continue
        if row.get('reason')!='compiler-artifact':continue
        package=row.get('target',{});profile=row.get('profile',{})
        if package.get('name')!='pluto_item_name_removal_tests' or profile.get('test') is not True:continue
        if package.get('kind')!=['lib'] or Path(package.get('src_path','')).resolve()!=(fixture/'lib.rs').resolve():
            raise ValueError('Wrong fixture test source')
        if profile.get('opt_level')!='0' or profile.get('debug_assertions') is not True:
            raise ValueError('Wrong test profile')
        binary=Path(row.get('executable') or '').resolve()
        if not binary.is_relative_to((target/'debug/deps').resolve()) or not binary.is_file() or not os.access(binary,os.X_OK):
            raise ValueError('Unexpected executable location')
        found.append(binary)
    if len(found)!=1:raise ValueError('Missing/duplicate fixture executable')
    return found[0]

def validate_list(log,names):
    actual=re.findall(r'^(\S+): test$',log,re.M)
    if len(actual)!=32 or len(set(actual))!=32 or set(actual)!=set(names):
        raise ValueError('Actual binary inventory differs')
    if re.findall(r'^(\d+) tests, 0 benchmarks$',log,re.M)!=['32']:
        raise ValueError('Unexpected binary inventory summary')

def execute(report):
    config=read_inputs();validate_inputs(config)
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    assert_event(config,event,os.environ)
    if git(ROOT,'rev-parse','HEAD')!=os.environ.get('GITHUB_SHA'):raise ValueError('Wrong checkout head')
    assert_git_clean(ROOT);original=checkout_seal(ROOT)
    work=Path(os.environ['RUNNER_TEMP'])/('item-name-removal-'+os.environ['GITHUB_SHA'])
    env=os.environ.copy()
    for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_NET_OFFLINE']:
        env.pop(key,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'target'),
       CARGO_BUILD_JOBS='1',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_TEST_OPT_LEVEL='0',
       CARGO_PROFILE_RELEASE_DEBUG='0',CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1')
    runner=Runner(work,env,report);runner.guard(reserve=2*1024**3)
    work.mkdir();Path(env['CARGO_HOME']).mkdir();Path(env['TMPDIR']).mkdir()
    gates=report['gates'];checkouts={};snapshots={};stage=None;stage_seal=None;fixture=None;fixture_seal=None
    generated=None;binary=None;binary_sha=None;cli=None;cli_sha=None;plan=None;plan_sha=None
    try:
        output=runner.run('runner-checks',[sys.executable,str(HERE/'test_runner.py'),'-v'])
        validate_python_checks(output,config['runner_tests']);gates['runner']={'passed':True,'tests':config['runner_tests']}
        for filename,count,gate in [('test_plan.py',12,'guard_checks'),('test_prepare.py',5,'preparer_checks')]:
            output=runner.run(gate,[sys.executable,str(BASE/'validation'/filename),'-v'])
            validate_python_checks(output,count);gates[gate]={'passed':True,'tests':count}
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal'],timeout=600)
        for name,pin in OFFICIAL.items():
            checkout=work/(name+'-git');exported=work/name
            runner.run(name+'-init',['git','init',str(checkout)])
            runner.run(name+'-origin',['git','-C',str(checkout),'remote','add','origin',pin['url']])
            runner.run(name+'-fetch',['git','-C',str(checkout),'fetch','--depth','1','origin',pin['revision']],timeout=600)
            runner.run(name+'-checkout',['git','-C',str(checkout),'checkout','--detach','FETCH_HEAD'])
            assert_git_clean(checkout)
            if git(checkout,'rev-parse','HEAD')!=pin['revision'] or git(checkout,'remote','get-url','origin')!=pin['url']:
                raise ValueError('Wrong official source')
            if sha((checkout/'Cargo.lock').read_bytes())!=pin['lock_sha256']:raise ValueError('Wrong official lock')
            checkouts[str(checkout)]=checkout_seal(checkout)
            runner.run(name+'-export',[sys.executable,str(HERE/'run.py'),'--export',str(checkout),pin['revision'],str(exported)])
            snapshots[str(exported)]=checkout_seal(exported)
            runner.run(name+'-locked-fetch',['cargo','fetch','--locked','--manifest-path',str(exported/'Cargo.toml')],timeout=900)
        gates['official_sources']={'passed':True,'pins':OFFICIAL}
        env['CARGO_NET_OFFLINE']='true'
        mixins=work/'mixins';official=work/'horizon'
        built=runner.run('mixins-build',['cargo','build','--release','--locked','--offline','-p','horizon-mixins-cli',
                        '--target-dir',str(work/'mixins-target'),'-vv'],cwd=mixins,timeout=1800)
        gates['mixins_compile']={'passed':True,'compilers':require_compile(built,'horizon_mixins')}
        cli=work/'mixins-target/release/horizon-mixins';cli_sha=sha(cli.read_bytes())
        plan=work/'plan.json';stage=work/'stage';candidate=BASE/'candidate'
        output=runner.run('actual-plan',[str(cli),'plan','--workspace',str(official),'--mods',str(candidate),'--out',str(plan)],timeout=180)
        counts=validate_cli(output);plan_sha=sha(plan.read_bytes())
        gates['plan']={'passed':True,'sha256':plan_sha,'counts':counts,'cli_sha256':cli_sha}
        runner.run('exclusive-guard',[sys.executable,str(BASE/'validation/check_plan.py'),str(plan),str(official)])
        gates['exclusive_guard']={'passed':True,'scope':'three targets plus all guest/receive extensions reserved'}
        output=runner.run('actual-apply',[str(cli),'apply','--workspace',str(official),'--mods',str(candidate),'--out',str(stage)],timeout=180)
        validate_cli(output);stage_seal=checkout_seal(stage)
        if json.loads((stage/'.horizon-mixins-stage.json').read_text())!=json.loads(plan.read_text()):
            raise ValueError('Apply independently produced a different plan')
        gates['apply']={'passed':True,'stage_seal_sha256':sha(json.dumps(stage_seal,sort_keys=True).encode())}
        fixture=work/'fixture';preparer=BASE/'validation/prepare_fixture.py'
        runner.run('prepare-applied-regions',[sys.executable,str(preparer),'--official',str(work/'horizon-git'),
                   '--stage',str(stage),'--plan',str(plan),'--output',str(fixture)])
        evidence=json.loads((fixture/'fixture-sources.json').read_text())
        if evidence['expected_tests']!=32 or evidence['result_groups']!=config['groups']:
            raise ValueError('Prepared groups changed')
        gates['applied_regions']={'passed':True,'fragments':evidence['fragments'],'modified_files':evidence['modified_files']}
        runner.run('metadata-resolve',[sys.executable,str(preparer),'--output',str(fixture),'--resolve'])
        runner.run('verify-before',[sys.executable,str(preparer),'--output',str(fixture),'--verify'])
        evidence=json.loads((fixture/'fixture-sources.json').read_text());fixture_seal=checkout_seal(fixture)
        gates['metadata']={'passed':True,'dependencies_sha256':sha(json.dumps(evidence['dependencies'],sort_keys=True).encode()),
                           'lock_sha256':sha((fixture/'Cargo.lock').read_bytes())}
        command=['cargo','test','--lib','--locked','--offline','--manifest-path',str(fixture/'Cargo.toml'),
                 '--no-run','--message-format=json-render-diagnostics','-vv']
        built=runner.run('native-codec-compile',command,timeout=1800)
        compilers=[]
        for name in ['horizon_core','horizon_client_core','horizon_items','horizon_protocol','horizon_world']:
            compilers+=require_compile(built,name)
        compilers+=require_compile(built,'pluto_item_name_removal_tests',True)
        binary=select_artifact(built,fixture,work/'target');binary_sha=sha(binary.read_bytes())
        gates['native_compile']={'passed':True,'compilers':compilers,'test_executable_sha256':binary_sha}
        generated=output_seal(work/'target')
        expected=[name for group in config['groups'] for name in group['tests']]
        listed=runner.run('native-inventory',[str(binary),'--list'],timeout=60)
        validate_list(listed,expected);gates['test_inventory']={'passed':True,'names':sorted(expected),'tests':32}
        output=runner.run('native32',[str(binary),'--test-threads=1'],timeout=180)
        validate_tests(output,expected,filtered=0)
        for group in config['groups']:
            gates[group['name']]={'passed':True,'tests':len(group['tests']),'names':group['tests']}
        print('VALIDATED_TEST_NAMES',json.dumps({'tests':32,'names':sorted(expected),'ignored':0,'filtered':0}),flush=True)
        runner.run('verify-after',[sys.executable,str(preparer),'--output',str(fixture),'--verify'])
        if set(gates)!=REQUIRED_GATES:raise ValueError('Missing required gate')
    except BaseException as exc:record_error(report,exc)
    finally:
        try:
            assert_git_clean(ROOT);assert_full_seal(ROOT,original)
            for path,before in checkouts.items():assert_git_clean(Path(path));assert_full_seal(Path(path),before)
            for path,before in snapshots.items():assert_full_seal(Path(path),before)
            if stage_seal is not None:assert_full_seal(stage,stage_seal)
            if fixture_seal is not None:assert_full_seal(fixture,fixture_seal)
            if generated is not None and output_seal(work/'target')!=generated:raise ValueError('Generated source drift')
            if binary is not None and sha(binary.read_bytes())!=binary_sha:raise ValueError('Native test binary drift')
            if cli is not None and sha(cli.read_bytes())!=cli_sha:raise ValueError('Mixins binary drift')
            if plan is not None and plan_sha is not None and sha(plan.read_bytes())!=plan_sha:raise ValueError('Plan drift')
            runner.guard();report['verified']=True
        except BaseException as exc:record_error(report,exc,'verification: ')
        report.update(minimum_free_bytes=runner.minimum_free,source_seal_sha256=sha(json.dumps(original,sort_keys=True).encode()),
          official_checkout_seals={p:sha(json.dumps(v,sort_keys=True).encode()) for p,v in checkouts.items()},
          stage_seal_sha256=sha(json.dumps(stage_seal,sort_keys=True).encode()) if stage_seal else None,
          fixture_seal_sha256=sha(json.dumps(fixture_seal,sort_keys=True).encode()) if fixture_seal else None,
          generated_output_seal_sha256=sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None)

def main():
    report={'head':os.environ.get('GITHUB_SHA'),'passed':False,'verified':False,'error':None,'gates':{},'phases':[],'floor_bytes':FLOOR,
      'scope':'Excluded candidate only: actual official plan/exclusive guard/apply, actual applied regions and codec32. Not installed common package, Game, LAN delivery, default ownership, universal TLS/depth safety or gameplay.'}
    try:
        execute(report)
        if report['error'] is None and set(report['gates'])!=REQUIRED_GATES:raise ValueError('Missing required gate')
    except BaseException as exc:record_error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as output:
            output.write('## Isolated item-name removal contract\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:record_error(report,exc,'summary: ');report['passed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)

if __name__=='__main__':
    if len(sys.argv)==5 and sys.argv[1]=='--export':export(Path(sys.argv[2]),sys.argv[3],Path(sys.argv[4]))
    elif len(sys.argv)==1:main()
    else:raise SystemExit('Invalid runner arguments')
