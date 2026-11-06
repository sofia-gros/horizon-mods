#!/usr/bin/env python3
"""Excluded crafting stored reader: real API153 and finite policy16, no startup hooks."""
import hashlib,json,math,os,re,shlex,shutil,signal,subprocess,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).resolve().parent
BASE=ROOT/'pluto-lib/experiments'
FLOOR=10*1024**3+128*1024**2
COMPILER_TARGETS={'horizon_core','horizon_client_core','horizon_items','horizon_world','pluto_inventory_api','pluto_crafting_stored_tests'}
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

LOG_LIMIT=64*1024**2

def sha(data): return hashlib.sha256(data).hexdigest()

def read_log(path):
    before=path.stat()
    if before.st_size>LOG_LIMIT:raise ValueError('Raw phase log exceeds bound')
    with path.open('rb') as stream:raw=stream.read(LOG_LIMIT+1)
    after=path.stat()
    if len(raw)>LOG_LIMIT or before.st_size!=len(raw) or after.st_size!=len(raw) or before.st_mtime_ns!=after.st_mtime_ns:
        raise ValueError('Raw phase log changed or exceeds bound')
    return raw

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
    def run(self, name, command, cwd=None, timeout=2400, separate_stderr=False):
        self.guard()
        path=self.work/(name+'.log');errpath=self.work/(name+'.stderr.log')
        print('PHASE_START',name,json.dumps(command),flush=True)
        started=time.monotonic();proc=None;error=None
        try:
            from contextlib import ExitStack
            with ExitStack() as stack:
                log=stack.enter_context(path.open('w'))
                err=stack.enter_context(errpath.open('w')) if separate_stderr else subprocess.STDOUT
                proc=subprocess.Popen(command,cwd=cwd or ROOT,env=self.env,stdout=log,stderr=err,start_new_session=True)
                try:
                    while proc.poll() is None:
                        self.guard()
                        if any(p.exists() and p.stat().st_size>LOG_LIMIT for p in [path,errpath] if separate_stderr or p==path):raise ValueError('Raw phase log exceeds bound')
                        if time.monotonic()-started>timeout:raise TimeoutError(name)
                        time.sleep(0.2)
                finally:
                    if proc.poll() is None:
                        try:os.killpg(proc.pid,signal.SIGTERM)
                        except ProcessLookupError:pass
                        deadline=time.monotonic()+1.0
                        while proc.poll() is None and time.monotonic()<deadline:time.sleep(0.05)
                    try:os.killpg(proc.pid,signal.SIGKILL)
                    except ProcessLookupError:pass
                    proc.wait(timeout=2);ensure_group_gone(proc.pid,name)
            self.guard()
            if proc.returncode!=0:raise RuntimeError(name+' exited '+str(proc.returncode))
            out=read_log(path).decode(errors='replace')
            return (out,read_log(errpath).decode(errors='replace')) if separate_stderr else out
        except BaseException as exc:error=exc;raise
        finally:
            streams={};read_error=None;texts={}
            for label,p in [('stdout',path)]+([('stderr',errpath)] if separate_stderr else []):
                try:
                    raw=read_log(p) if p.exists() else b''
                    streams[label]={'raw_sha256':sha(raw),'raw_bytes':len(raw)};texts[label]=raw.decode(errors='replace')
                except BaseException as exc:
                    read_error=type(exc).__name__+': '+str(exc)
                    streams[label]={'raw_sha256':None,'raw_bytes':p.stat().st_size if p.exists() else None,'error':read_error};texts[label]=''
            phase={'name':name,'seconds':round(time.monotonic()-started,3),'exit_code':proc.returncode if proc is not None else None,
                'streams':streams,'raw_sha256':streams['stdout']['raw_sha256'],'raw_bytes':streams['stdout']['raw_bytes'],'raw_read_error':read_error,
                'error':None if error is None else type(error).__name__+': '+str(error)}
            self.report['phases'].append(phase)
            try:
                if read_error:raise ValueError(read_error)
                phase['compilers']=compiler_records(texts.get('stderr',texts['stdout']))
                for text in texts.values():print_compact_log(text,error is not None)
            except BaseException as exc:phase['summary_error']=type(exc).__name__+': '+str(exc);raise
            finally:print('PHASE_RECEIPT',json.dumps(phase,sort_keys=True),flush=True)

def compiler_records(log):
    records=[]
    for line in log.splitlines():
        if 'Running `' not in line or 'rustc' not in line: continue
        argv=shlex.split(line.split('Running `',1)[1].rsplit('`',1)[0])
        is_env=lambda token: re.match(r'^[A-Za-z_][A-Za-z0-9_]*=',token) is not None
        commands=[i for i,token in enumerate(argv) if not is_env(token) and Path(token).name=='rustc']
        if not commands:continue  # A build-script command may itself carry RUSTC=.
        if len(commands)!=1:raise ValueError('Ambiguous actual rustc executable')
        at=commands[0]
        if any(not is_env(token) for token in argv[:at]):raise ValueError('Unexpected wrapper before rustc')
        args=argv[at+1:]
        if '--crate-name' not in args:continue
        if args.count('--crate-name')!=1 or args.index('--crate-name')+1==len(args):raise ValueError('Malformed crate-name argument')
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
BRANCH='checkpoint/plutolib-crafting-stored-ci-20261106'
WORKFLOW='.github/workflows/crafting-stored-wip.yml'
SOURCE_BASE='17f44dc36fc2df28547328aa8554286ae965b208'
OFFICIAL={'url':'https://github.com/horizon-rust/horizon.git','revision':'41f822b659789780f58d32c63ddfb6887ea1b8bf','lock_sha256':'0bc4234f3001d44df9d46fbb06df331d3f53b1af6b613ec9082baaec3d93c62f'}
API='pluto-lib/mixins/pluto_native_creative/inventory-api'
PREPARER=BASE/'crafting-stored-fixture'
CANDIDATE=BASE/'crafting-stored-facts'
GROUPS=[
 {'name':'api153','package':'pluto-inventory-api','crate':'pluto_inventory_api','source':'inventory-api/src/lib.rs','count':153},
 {'name':'finite16','package':'pluto-crafting-stored-tests','crate':'pluto_crafting_stored_tests','source':'fixture/lib.rs','count':16},
]
REQUIRED_GATES={'runner','renderer','preparer','official_sources','metadata','native_compile','api_library_compile',*[g['name']+'_'+k for g in GROUPS for k in ['compile','inventory','tests']]}

def validate_inputs(config):
    if (config['repository'],config['branch'],config['base'])!=(REPOSITORY,BRANCH,SOURCE_BASE):raise ValueError('Wrong repository, dedicated branch or source base')
    if config['official']!=OFFICIAL or config['groups']!=GROUPS:raise ValueError('Wrong official target or proof groups')
    meta=json.loads((PREPARER/'inputs.json').read_text())
    expected={API+'/'+n:h for n,h in meta['api_base_sha256'].items()}
    expected.update({'pluto-lib/experiments/crafting-stored-facts/'+n:h for n,h in meta['candidate_sha256'].items()})
    expected.update({'pluto-lib/experiments/crafting-stored-fixture/'+n:h for n,h in meta['helper_sha256'].items()})
    expected['pluto-lib/experiments/crafting-stored-fixture/inputs.json']=sha((PREPARER/'inputs.json').read_bytes())
    for n in ['run.py','test_runner.py','README.md']:expected['pluto-lib/experiments/crafting-stored-ci/'+n]=sha((HERE/n).read_bytes())
    expected[WORKFLOW]=sha((ROOT/WORKFLOW).read_bytes())
    if config['source_sha256']!=expected:raise ValueError('Frozen input file-set/hashes differ')
    for path,digest in expected.items():
        p=ROOT/path
        if p.is_symlink() or not p.is_file() or sha(p.read_bytes())!=digest:raise ValueError('Frozen source changed: '+path)
    if config['tests']!={'api153':meta['api_expected_tests'],'finite16':meta['fixture_expected_tests']}:raise ValueError('Complete test inventories changed')
    old,new=meta['api_existing_tests'],meta['api_added_tests']
    if len(old)!=147 or len(new)!=6 or len(set(old+new))!=153:raise ValueError('Old/new API inventory changed')
    if config['test_groups']!={'api_existing147':old,'api_added6':new}:raise ValueError('API subgroups changed')
    for group in GROUPS:
        names=config['tests'][group['name']]
        if len(names)!=group['count'] or len(set(names))!=len(names):raise ValueError('Wrong unique test count')
    if config['pure_counts']!={'runner':31,'preparer':21,'renderer':13}:raise ValueError('Pure check counts changed')

def assert_event(config,event,env):
    if (env.get('GITHUB_ACTIONS')!='true' or env.get('GITHUB_REPOSITORY')!=REPOSITORY or
        env.get('GITHUB_EVENT_NAME')!='push' or env.get('GITHUB_REF')!='refs/heads/'+BRANCH or
        event.get('repository',{}).get('private') is not False):
        raise ValueError('Require public exact dedicated branch push')

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

def validate_metadata_receipt(metadata,evidence):
    packages=metadata.get('packages',[])
    projected=[{k:p[k] for k in ['name','id','manifest_path','source']} for p in packages]
    if projected!=evidence['metadata_packages']:raise ValueError('Post-resolution package identities changed')
    if len({p['id'] for p in packages})!=len(packages):raise ValueError('Duplicate metadata identity')
    for p in packages:
        if not isinstance(p.get('version'),str) or not re.fullmatch(r'[0-9A-Za-z.+_-]+',p['version']):raise ValueError('Invalid version prefix')
        if not re.fullmatch(r'[0-9A-Za-z_-]+',p['name']):raise ValueError('Invalid name prefix')
    return packages

def artifact_rows(stdout,packages):
    def unique_object(pairs):
        value={}
        for key,item in pairs:
            if key in value:raise ValueError('Duplicate JSON field: '+key)
            value[key]=item
        return value
    def no_constant(value):raise ValueError('Non-JSON numeric constant: '+value)
    def finite_float(value):
        number=float(value)
        if not math.isfinite(number):raise ValueError('Nonfinite JSON number')
        return number
    def reject_line(index,line,reason):
        raw=line.encode()
        raise ValueError(reason+' at stdout line '+str(index)+' bytes='+str(len(raw))+' sha256='+sha(raw)+' excerpt='+line[:320])
    prefixes={}
    for p in packages:
        key=(p['name'],p['version'])
        if sum((q['name'],q['version'])==key for q in packages)==1 and any(t.get('kind')==['custom-build'] for t in p.get('targets',[])):
            prefixes['['+p['name']+' '+p['version']+'] ']=p['id']
    ids={p['id'] for p in packages};rows=[];notices={}
    for index,line in enumerate(stdout.splitlines(),1):
        if not line.strip():continue
        if line.startswith('{'):
            try:row=json.loads(line,object_pairs_hook=unique_object,parse_constant=no_constant,parse_float=finite_float)
            except (json.JSONDecodeError,ValueError):reject_line(index,line,'Malformed Cargo JSON')
            if not isinstance(row,dict) or row.get('reason') not in {'compiler-artifact','compiler-message','build-script-executed','build-finished'}:raise ValueError('Unknown Cargo JSON at line '+str(index))
            if row['reason']!='build-finished' and row.get('package_id') not in ids:raise ValueError('JSON package identity outside exact metadata')
            rows.append(row);continue
        prefix=next((p for p in prefixes if line.startswith(p)),None)
        if prefix is None:reject_line(index,line,'Unknown Cargo output')
        notices[prefix]=notices.get(prefix,0)+1
    finished=[r for r in rows if r['reason']=='build-finished']
    if len(finished)!=1 or finished[0].get('success') is not True:raise ValueError('Missing/duplicate/failed Cargo build-finished')
    return rows,notices

def select_artifact(rows,expected_source,target,crate,package_id):
    found=[]
    for row in rows:
        if row.get('reason')!='compiler-artifact':continue
        package=row.get('target',{});profile=row.get('profile',{})
        if package.get('name')!=crate or profile.get('test') is not True:continue
        if row.get('package_id')!=package_id:raise ValueError('Wrong artifact package identity')
        if row.get('fresh') is not False:raise ValueError('Fixture artifact was not compiled in this run')
        if package.get('kind')!=['lib'] or Path(package.get('src_path','')).resolve()!=expected_source.resolve():raise ValueError('Wrong fixture source')
        if profile.get('opt_level')!='0' or profile.get('debug_assertions') is not True:raise ValueError('Wrong fixture profile')
        candidate=Path(row.get('executable') or '');binary=candidate.resolve()
        if candidate.is_symlink() or not binary.is_relative_to((target/'debug/deps').resolve()) or not binary.is_file() or not os.access(binary,os.X_OK):raise ValueError('Wrong binary path/kind')
        found.append(binary)
    if len(found)!=1:raise ValueError('Missing/duplicate fixture artifact')
    return found[0]

def validate_list(log,names):
    actual=re.findall(r'^(\S+): test$',log,re.M)
    if len(actual)!=len(names) or len(set(actual))!=len(names) or set(actual)!=set(names):raise ValueError('Binary inventory differs')
    if re.findall(r'^(\d+) tests, 0 benchmarks$',log,re.M)!=[str(len(names))]:raise ValueError('Binary count differs')

def assert_old_outputs_unchanged(before, after):
    if any(name not in after or after[name]!=value for name,value in before.items()):
        raise ValueError('Previously generated native outputs changed during second compile')

def execute(report):
    config=read_inputs();validate_inputs(config)
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text());assert_event(config,event,os.environ)
    if git(ROOT,'rev-parse','HEAD')!=os.environ.get('GITHUB_SHA'):raise ValueError('Wrong checkout head')
    assert_git_clean(ROOT);original=checkout_seal(ROOT)
    work=Path(os.environ['RUNNER_TEMP'])/('crafting-stored-'+os.environ['GITHUB_SHA'])
    env=os.environ.copy()
    for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_NET_OFFLINE']:
        env.pop(key,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'target'),
        CARGO_BUILD_JOBS='1',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_TEST_OPT_LEVEL='0',
        PLUTO_FIXTURE_CONTROLLER='crafting-stored-v1',CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1')
    runner=Runner(work,env,report);runner.guard(reserve=2*1024**3)
    work.mkdir();Path(env['CARGO_HOME']).mkdir();Path(env['TMPDIR']).mkdir()
    gates=report['gates'];official=work/'official';workspace=work/'workspace';preparer=PREPARER/'prepare.py'
    native_seal=None;fixture_seal=None;binary_seals={};generated=None;before_outputs=None
    try:
        for name,path in [('runner',HERE/'test_runner.py'),('preparer',PREPARER/'test_prepare.py'),('renderer',CANDIDATE/'test_source.py')]:
            out=runner.run(name+'-checks',[sys.executable,str(path),'-v'])
            validate_python_checks(out,config['pure_counts'][name]);gates[name]={'passed':True,'tests':config['pure_counts'][name]}
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
        runner.run('workspace-prepare',[sys.executable,str(preparer),'--prepare','--source',str(ROOT),'--official',str(official),'--output',str(workspace)])
        fixture_seal=checkout_seal(workspace)
        runner.run('workspace-metadata',[sys.executable,str(preparer),'--output',str(workspace),'--resolve'])
        runner.run('workspace-verify-before',[sys.executable,str(preparer),'--output',str(workspace),'--verify'])
        evidence=json.loads((workspace/'fixture-sources.json').read_text())
        if not evidence['resolved'] or evidence['api_tests']!=config['tests']['api153'] or evidence['fixture_tests']!=config['tests']['finite16']:raise ValueError('Prepared workspace proof identity differs')
        fixture_seal=checkout_seal(workspace)
        identity_out,identity_err=runner.run('workspace-identities',['cargo','metadata','--locked','--offline','--format-version=1','--manifest-path',str(workspace/'Cargo.toml')],cwd=workspace,separate_stderr=True)
        packages=validate_metadata_receipt(json.loads(identity_out),evidence)
        assert_full_seal(workspace,fixture_seal)
        gates['metadata']={'passed':True,'lock_sha256':sha((workspace/'Cargo.lock').read_bytes()),'packages':evidence['metadata_packages'],'scope':'One virtual workspace with exactly one real API and one finite fixture identity'}
        for group in GROUPS:
            name=group['name'];expected_source=workspace/group['source']
            stdout,stderr=runner.run(name+'-compile',['cargo','test','-p',group['package'],'--lib','--locked','--offline','--manifest-path',str(workspace/'Cargo.toml'),
                '--no-run','--message-format=json-render-diagnostics','-vv'],cwd=workspace,timeout=1800,separate_stderr=True)
            rows,notices=artifact_rows(stdout,packages)
            if name=='finite16':
                records=[]
                for crate in ['horizon_core','horizon_client_core','horizon_items','horizon_world']:
                    records+=require_compile(stderr,crate,official/'crates'/crate.replace('_','-')/'src/lib.rs',workspace)
                gates['native_compile']={'passed':True,'owner':name,'compilers':records}
                records=require_compile(stderr,'pluto_inventory_api',workspace/'inventory-api/src/lib.rs',workspace)
                gates['api_library_compile']={'passed':True,'owner':name,'compilers':records}
            records=require_compile(stderr,group['crate'],expected_source,workspace,True)
            package_id=next(p['id'] for p in packages if p['name']==group['package'])
            binary=select_artifact(rows,expected_source,work/'target',group['crate'],package_id);binary_seals[name]=(binary,sha(binary.read_bytes()))
            gates[name+'_compile']={'passed':True,'compilers':records,'binary_sha256':binary_seals[name][1],'stdout_notices':notices}
            assert_full_seal(workspace,fixture_seal);assert_full_seal(official,native_seal)
            after=output_seal(work/'target')
            if before_outputs is not None:assert_old_outputs_unchanged(before_outputs,after)
            before_outputs=after
        if len({path for path,digest in binary_seals.values()})!=2:raise ValueError('Expected two separate test binaries')
        generated=output_seal(work/'target')
        # No Cargo follows this point: execute only the two sealed test binaries.
        for group in GROUPS:
            name=group['name'];binary,digest=binary_seals[name];names=config['tests'][name]
            if sha(binary.read_bytes())!=digest:raise ValueError('Binary changed before execution')
            listed=runner.run(name+'-inventory',[str(binary),'--list'],timeout=60);validate_list(listed,names)
            gates[name+'_inventory']={'passed':True,'names':names,'tests':len(names)}
            out=runner.run(name+'-tests',[str(binary),'--test-threads=1'],timeout=180);validate_tests(out,names,filtered=0)
            gates[name+'_tests']={'passed':True,'names':names,'tests':len(names),'ignored':0,'filtered':0}
            if name=='api153':gates[name+'_tests']['groups']={k:{'passed':True,'names':v,'tests':len(v)} for k,v in config['test_groups'].items()}
            runner.run(name+'-verify-after',[sys.executable,str(preparer),'--output',str(workspace),'--verify'])
            if sha(binary.read_bytes())!=digest or output_seal(work/'target')!=generated:raise ValueError('Binary/generated source changed during tests')
        if set(gates)!=REQUIRED_GATES:raise ValueError('Missing required gates')
    except BaseException as exc:record_error(report,exc)
    finally:
        try:
            assert_git_clean(ROOT);assert_full_seal(ROOT,original)
            if native_seal is not None:assert_git_clean(official);assert_full_seal(official,native_seal)
            if fixture_seal is not None:assert_full_seal(workspace,fixture_seal)
            if generated is not None and output_seal(work/'target')!=generated:raise ValueError('Generated source drift')
            for binary,digest in binary_seals.values():
                if binary.is_symlink() or sha(binary.read_bytes())!=digest:raise ValueError('Binary drift')
            runner.guard();report['verified']=True
        except BaseException as exc:record_error(report,exc,'verification: ')
        report.update(minimum_free_bytes=runner.minimum_free,source_seal_sha256=sha(json.dumps(original,sort_keys=True).encode()),
            official_seal_sha256=sha(json.dumps(native_seal,sort_keys=True).encode()) if native_seal is not None else None,
            workspace_seal_sha256=sha(json.dumps(fixture_seal,sort_keys=True).encode()) if fixture_seal is not None else None,
            generated_output_seal_sha256=sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None)
        report['fixtures']={g['name']:{'tests':g['count'],'tests_passed':g['name']+'_tests' in gates,'verified':report['verified'],
            'gates':{k:v for k,v in gates.items() if k.startswith(g['name']+'_')}} for g in GROUPS}

def main():
    report={'head':os.environ.get('GITHUB_SHA'),'passed':False,'verified':False,'error':None,'gates':{},'phases':[],'floor_bytes':FLOOR,
        'scope':'Excluded API153 plus finite stored-policy16 using actual 41f native dependencies. No Mixins plan/apply, constructor/bootstrap, installed Game adapter, output/remainder, placement transaction or production activation.'}
    try:
        execute(report)
        if report['error'] is None and set(report['gates'])!=REQUIRED_GATES:raise ValueError('Missing required gate')
    except BaseException as exc:record_error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:f.write('## Crafting stored-value admission\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:record_error(report,exc,'summary: ');report['passed']=False
    for name,fixture in report.get('fixtures',{}).items():print('FIXTURE_RECEIPT',json.dumps({'name':name,**fixture},sort_keys=True),flush=True)
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)

if __name__=='__main__':main()
