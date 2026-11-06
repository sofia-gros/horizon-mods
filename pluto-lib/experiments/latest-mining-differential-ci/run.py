#!/usr/bin/env python3
"""Latest mining scalar policies: 24 existing plus 12 extra cases, no Game hooks."""
import hashlib,json,os,re,shlex,shutil,signal,subprocess,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).resolve().parent
BASE=ROOT/'pluto-lib/experiments'
FLOOR=10*1024**3+128*1024**2
COMPILER_TARGETS={'horizon_core','horizon_client_core','horizon_items','horizon_world','pluto_latest_mining_differential_tests'}
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
    proc=subprocess.Popen(['git','-C',str(root),*args],text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
    try:
        out,err=proc.communicate(timeout=30)
        if proc.returncode:raise ValueError('Read-only Git failed: '+err[-2000:])
        return out.strip()
    finally:
        try:os.killpg(proc.pid,signal.SIGKILL)
        except ProcessLookupError:pass
        proc.wait(timeout=2);ensure_group_gone(proc.pid,'read-only-git')

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

REPOSITORY='sofia-gros/horizon-mods'
BRANCH='checkpoint/plutolib-latest-mining-differential-ci-20261106'
SOURCE_BASE='5d343c174ab38b0571c4f2e18505a655646ddfe0'
MOD_PIN='08afa992d5091e03360f3473ea93fd17aa793987'
MOD_URL='https://github.com/sofia-gros/horizon-mods.git'
OFFICIAL={'url':'https://github.com/horizon-rust/horizon.git','revision':'f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd','lock_sha256':'0bc4234f3001d44df9d46fbb06df331d3f53b1af6b613ec9082baaec3d93c62f'}
GROUPS=[
 {'name':'mining36','preparer':'latest-mining-fixture','crate':'pluto_latest_mining_differential_tests','prefix':'','count':36,'python':20},
]
FROZEN_FILES={
 'latest-mining-differential':['README.md','implementation-review.md','inputs.json','prepare_sources.py','regression_tests.rs','source-verification.json','test_prepare.py'],
 'latest-mining-fixture':['README.md','inputs.json','prepare.py','test_prepare.py'],
 'latest-mining-differential-ci':['README.md','run.py','test_runner.py'],
}
REQUIRED_INPUTS={'pluto-lib/experiments/'+directory+'/'+name for directory,names in FROZEN_FILES.items() for name in names}
REQUIRED_INPUTS.add('.github/workflows/latest-mining-differential-wip.yml')
REQUIRED_GATES={'runner','renderer','official_sources','mod_oracle','shared_native_compile',*[g['name']+'_'+k for g in GROUPS for k in ['preparer','metadata','compile','inventory','tests']]}

def validate_inputs(config):
    if (config['repository'],config['branch'],config['base'])!=(REPOSITORY,BRANCH,SOURCE_BASE):
        raise ValueError('Wrong repository, dedicated branch or base')
    if config['official']!=OFFICIAL:raise ValueError('Wrong official target')
    if config['groups']!=GROUPS:raise ValueError('Fixture identities/scopes changed')
    if set(config['source_sha256'])!=REQUIRED_INPUTS:raise ValueError('Frozen source file-set changed')
    for path,digest in config['source_sha256'].items():
        f=ROOT/path
        if f.is_symlink() or sha(f.read_bytes())!=digest:raise ValueError('Frozen source changed: '+path)
    meta=json.loads((BASE/'latest-mining-fixture/inputs.json').read_text())
    additional=['regression_tests::'+n for n in re.findall(r'#\[test\]\s*fn ([a-z0-9_]+)', (BASE/'latest-mining-differential/regression_tests.rs').read_text())]
    existing=meta['existing_tests']
    if len(existing)!=24 or len(set(existing))!=24 or len(additional)!=12 or len(set(additional))!=12 or set(existing)&set(additional):
        raise ValueError('Existing and added test inventories differ')
    if meta['additional_tests']!=additional or meta['expected_tests']!=existing+additional or config['tests']!={'mining36':existing+additional}:
        raise ValueError('Full binary inventory changed')
    if config['test_groups']!={'existing24':existing,'additional12':additional}:raise ValueError('Test subgroups changed')


def assert_event(config,event,env):
    if (env.get('GITHUB_ACTIONS')!='true' or env.get('GITHUB_REPOSITORY')!=REPOSITORY or
        env.get('GITHUB_EVENT_NAME')!='push' or env.get('GITHUB_REF')!='refs/heads/'+BRANCH or
        event.get('repository',{}).get('private') is not False):
        raise ValueError('Require exact public dedicated push branch')

def output_seal(target):
    result={}
    for out in sorted(target.glob('debug/build/*/out')):
        tree=checkout_seal(out)
        if any(row['kind']=='symlink' for row in tree.values()):raise ValueError('Generated source link')
        result[out.relative_to(target).as_posix()]=tree
    return result

def require_compile(log,crate,expected_source,cwd,test=False):
    selected=[r for r in compiler_records(log) if r['crate']==crate and r['test']==test]
    if not selected or (test and len(selected)!=1):raise ValueError('Missing/duplicate actual compiler: '+crate)
    for record in selected:
        paths=[(cwd/Path(s)).resolve() for s in record['source']]
        if paths!=[expected_source.resolve()]:raise ValueError('Actual compiler source identity differs: '+crate)
    return selected

def select_artifact(log,fixture,target,crate):
    found=[]
    for line in log.splitlines():
        if not line.startswith('{'):continue
        try:row=json.loads(line)
        except json.JSONDecodeError:continue
        if row.get('reason')!='compiler-artifact':continue
        package=row.get('target',{});profile=row.get('profile',{})
        if package.get('name')!=crate or profile.get('test') is not True:continue
        if row.get('fresh') is not False:raise ValueError('Fixture artifact was not compiled in this run')
        if package.get('kind')!=['lib'] or Path(package.get('src_path','')).resolve()!=(fixture/'lib.rs').resolve():raise ValueError('Wrong fixture source')
        if profile.get('opt_level')!='0' or profile.get('debug_assertions') is not True:raise ValueError('Wrong fixture profile')
        candidate=Path(row.get('executable') or '')
        binary=candidate.resolve()
        if candidate.is_symlink() or not binary.is_relative_to((target/'debug/deps').resolve()) or not binary.is_file() or not os.access(binary,os.X_OK):raise ValueError('Wrong binary path/kind')
        found.append(binary)
    if len(found)!=1:raise ValueError('Missing/duplicate fixture artifact')
    return found[0]

def validate_list(log,names):
    actual=re.findall(r'^(\S+): test$',log,re.M)
    if len(actual)!=len(names) or len(set(actual))!=len(names) or set(actual)!=set(names):raise ValueError('Binary inventory differs')
    if re.findall(r'^(\d+) tests, 0 benchmarks$',log,re.M)!=[str(len(names))]:raise ValueError('Binary count differs')

def execute(report):
    config=read_inputs();validate_inputs(config)
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text());assert_event(config,event,os.environ)
    if git(ROOT,'rev-parse','HEAD')!=os.environ.get('GITHUB_SHA'):raise ValueError('Wrong checkout head')
    assert_git_clean(ROOT);original=checkout_seal(ROOT)
    work=Path(os.environ['RUNNER_TEMP'])/('latest-mining-differential-'+os.environ['GITHUB_SHA'])
    env=os.environ.copy()
    for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_NET_OFFLINE']:
        env.pop(key,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'target'),
        CARGO_BUILD_JOBS='1',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_TEST_OPT_LEVEL='0',
        PLUTO_FIXTURE_CONTROLLER='latest-mining-differential-v1',CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1')
    runner=Runner(work,env,report);runner.guard(reserve=2*1024**3)
    work.mkdir();Path(env['CARGO_HOME']).mkdir();Path(env['TMPDIR']).mkdir()
    gates=report['gates'];official=work/'official';oracle=work/'mod-oracle';native_seal=None;fixtures={};binary_seals={};generated=None
    try:
        out=runner.run('runner-checks',[sys.executable,str(HERE/'test_runner.py'),'-v'])
        validate_python_checks(out,config['runner_tests']);gates['runner']={'passed':True,'tests':config['runner_tests']}
        for group in GROUPS:
            out=runner.run(group['name']+'-preparer-checks',[sys.executable,str(BASE/group['preparer']/'test_prepare.py'),'-v'])
            validate_python_checks(out,group['python']);gates[group['name']+'_preparer']={'passed':True,'tests':group['python']}
        out=runner.run('renderer-checks',[sys.executable,str(BASE/'latest-mining-differential/test_prepare.py'),'-v'])
        validate_python_checks(out,6);gates['renderer']={'passed':True,'tests':6}
        runner.run('mod-oracle-init',['git','init',str(oracle)])
        runner.run('mod-oracle-origin',['git','-C',str(oracle),'remote','add','origin',MOD_URL])
        runner.run('mod-oracle-fetch',['git','-C',str(oracle),'fetch','--depth','1','origin',MOD_PIN],timeout=600)
        if git(oracle,'rev-parse','FETCH_HEAD')!=MOD_PIN or git(oracle,'remote','get-url','origin')!=MOD_URL:raise ValueError('Wrong Mod oracle')
        gates['mod_oracle']={'passed':True,'revision':MOD_PIN,'url':MOD_URL,'scope':'Frozen source objects only; no checkout/runtime'}
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal'],timeout=600)
        runner.run('official-init',['git','init',str(official)])
        runner.run('official-origin',['git','-C',str(official),'remote','add','origin',OFFICIAL['url']])
        runner.run('official-fetch',['git','-C',str(official),'fetch','--depth','1','origin',OFFICIAL['revision']],timeout=600)
        runner.run('official-checkout',['git','-C',str(official),'checkout','--detach','FETCH_HEAD'])
        assert_git_clean(official)
        if git(official,'rev-parse','HEAD')!=OFFICIAL['revision'] or git(official,'remote','get-url','origin')!=OFFICIAL['url']:raise ValueError('Wrong official checkout')
        if sha((official/'Cargo.lock').read_bytes())!=OFFICIAL['lock_sha256']:raise ValueError('Wrong official lock')
        native_seal=checkout_seal(official);gates['official_sources']={'passed':True,'pin':OFFICIAL}
        runner.run('official-locked-fetch',['cargo','fetch','--locked','--manifest-path',str(official/'Cargo.toml')],timeout=900)
        env['CARGO_NET_OFFLINE']='true'
        for group in GROUPS:
            name=group['name'];fixture=work/name;preparer=BASE/group['preparer']/'prepare.py'
            runner.run(name+'-prepare',[sys.executable,str(preparer),'--official',str(official),'--oracle',str(oracle),'--output',str(fixture)])
            fixtures[name]={'path':fixture,'preparer':preparer,'seal':checkout_seal(fixture)}
            runner.run(name+'-metadata',[sys.executable,str(preparer),'--output',str(fixture),'--resolve'])
            runner.run(name+'-verify-before',[sys.executable,str(preparer),'--output',str(fixture),'--verify'])
            evidence=json.loads((fixture/'fixture-sources.json').read_text())
            expected=[group['prefix']+n for n in evidence['expected_tests']]
            if not evidence['resolved'] or expected!=config['tests'][name]:raise ValueError('Prepared fixture identity differs')
            fixtures[name]={'path':fixture,'preparer':preparer,'seal':checkout_seal(fixture)}
            gates[name+'_metadata']={'passed':True,'lock_sha256':sha((fixture/'Cargo.lock').read_bytes()),'dependencies_sha256':sha(json.dumps(evidence['dependencies'],sort_keys=True).encode())}
        # Compile the complete unfiltered test harness once with a cold native graph.
        for index,group in enumerate(GROUPS):
            name=group['name'];fixture=fixtures[name]['path']
            out=runner.run(name+'-compile',['cargo','test','--lib','--locked','--offline','--manifest-path',str(fixture/'Cargo.toml'),
                '--no-run','--message-format=json-render-diagnostics','-vv'],cwd=fixture,timeout=1800)
            native_records=[]
            if index==0:
                for crate in ['horizon_core','horizon_client_core','horizon_items','horizon_world']:
                    native_records+=require_compile(out,crate,official/'crates'/crate.replace('_','-')/'src/lib.rs',fixture)
                gates['shared_native_compile']={'passed':True,'owner':name,'compilers':native_records,'scope':'One actual cold native graph plus one complete scalar-policy harness; not Game adapter typing'}
            test_records=require_compile(out,group['crate'],fixture/'lib.rs',fixture,True)
            binary=select_artifact(out,fixture,work/'target',group['crate']);binary_seals[name]=(binary,sha(binary.read_bytes()))
            gates[name+'_compile']={'passed':True,'compilers':test_records,'other_native_compilers':compiler_records(out),'binary_sha256':binary_seals[name][1]}
            assert_full_seal(fixture,fixtures[name]['seal']);assert_full_seal(official,native_seal)
            if index==0:generated=output_seal(work/'target')
            elif output_seal(work/'target')!=generated:raise ValueError('Shared native generated source changed between harnesses')
        if len({path for path,digest in binary_seals.values()})!=1:raise ValueError('Expected one native test binary')
        generated=output_seal(work/'target')
        # No Cargo follows this point: execute only sealed test binaries.
        for group in GROUPS:
            name=group['name'];binary,digest=binary_seals[name];expected=config['tests'][name]
            if sha(binary.read_bytes())!=digest:raise ValueError('Binary changed before execution')
            listed=runner.run(name+'-inventory',[str(binary),'--list'],timeout=60);validate_list(listed,expected)
            gates[name+'_inventory']={'passed':True,'names':expected,'tests':len(expected)}
            out=runner.run(name+'-tests',[str(binary),'--test-threads=1'],timeout=180);validate_tests(out,expected,filtered=0)
            gates[name+'_tests']={'passed':True,'names':expected,'tests':len(expected),'ignored':0,'filtered':0,'groups':{k:{'passed':True,'names':v,'tests':len(v)} for k,v in config['test_groups'].items()}}
            runner.run(name+'-verify-after',[sys.executable,str(fixtures[name]['preparer']),'--output',str(fixtures[name]['path']),'--verify'])
            if sha(binary.read_bytes())!=digest or output_seal(work/'target')!=generated:raise ValueError('Binary/generated source changed after test')
        if set(gates)!=REQUIRED_GATES:raise ValueError('Missing required gates')
    except BaseException as exc:record_error(report,exc)
    finally:
        try:
            assert_git_clean(ROOT);assert_full_seal(ROOT,original)
            if native_seal is not None:assert_git_clean(official);assert_full_seal(official,native_seal)
            for record in fixtures.values():assert_full_seal(record['path'],record['seal'])
            if generated is not None and output_seal(work/'target')!=generated:raise ValueError('Generated source drift')
            for binary,digest in binary_seals.values():
                if binary.is_symlink() or sha(binary.read_bytes())!=digest:raise ValueError('Binary drift')
            runner.guard();report['verified']=True
        except BaseException as exc:record_error(report,exc,'verification: ')
        report.update(minimum_free_bytes=runner.minimum_free,source_seal_sha256=sha(json.dumps(original,sort_keys=True).encode()),
            official_seal_sha256=sha(json.dumps(native_seal,sort_keys=True).encode()) if native_seal is not None else None,
            generated_output_seal_sha256=sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None)
        report['fixtures']={g['name']:{'tests':g['count'],'tests_passed':g['name']+'_tests' in gates,'verified':report['verified'],
            'source_seal_sha256':sha(json.dumps(fixtures[g['name']]['seal'],sort_keys=True).encode()) if g['name'] in fixtures else None,
            'gates':{key:value for key,value in gates.items() if key.startswith(g['name']+'_')}} for g in GROUPS}

def main():
    report={'head':os.environ.get('GITHUB_SHA'),'passed':False,'verified':False,'error':None,'gates':{},'phases':[],'floor_bytes':FLOOR,
        'scope':'Main08afa mining rules/math: 24 existing plus 12 scalar differential cases with actual f4dc native dependencies. No Mixins plan/apply, Game adapter typing, full host/guest actor path, permission, Wire identity or production activation.'}
    try:
        execute(report)
        if report['error'] is None and set(report['gates'])!=REQUIRED_GATES:raise ValueError('Missing required gate')
    except BaseException as exc:record_error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:f.write('## Latest mining differential contracts\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:record_error(report,exc,'summary: ');report['passed']=False
    for name,fixture in report.get('fixtures',{}).items():print('FIXTURE_RECEIPT',json.dumps({'name':name,**fixture},sort_keys=True),flush=True)
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)

if __name__=='__main__':main()
