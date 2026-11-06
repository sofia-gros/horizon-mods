"""Prepared, UNRUN bounded proof. A separate serial grant is mandatory."""
from pathlib import Path
import hashlib,json,os,re,signal,subprocess,sys,time
OUT=Path(__file__).resolve().parent
assert str(OUT).startswith('/tmp/'), 'Materialize this proof harness in the authorized tmpfs first'
BASE=Path('/workspace/scratch/c7b540ff3393')
REPO=BASE/'upgrade-official-baseline-20261105'
PLAN=json.loads((OUT/'plan.json').read_text())
HOST=Path(PLAN['official_path']); SOURCE=OUT/'source'
FLOOR=PLAN['normal_floor']; START=FLOOR+PLAN['required_start_margin']
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
free=lambda:os.statvfs(BASE).f_bavail*os.statvfs(BASE).f_frsize

def official():
    assert subprocess.check_output(['git','-C',str(HOST),'rev-parse','HEAD'],text=True).strip()==PLAN['official_revision']
    assert not subprocess.check_output(['git','-C',str(HOST),'status','--porcelain','--untracked-files=no'],text=True)
    assert h(HOST/'Cargo.lock')==PLAN['official_lock_sha256']
    paths=subprocess.check_output(['git','-C',str(HOST),'ls-files','-z']).decode().split('\0')
    return {p:h(HOST/p) for p in paths if p}

def source_seal():
    return {p:h(SOURCE/p) for p in PLAN['files']}

def stop(process):
    for sig in (signal.SIGINT,signal.SIGTERM,signal.SIGKILL):
        try:os.killpg(process.pid,sig)
        except ProcessLookupError:return
        try:process.wait(timeout=1)
        except subprocess.TimeoutExpired:continue
        try:os.killpg(process.pid,signal.SIGKILL)
        except ProcessLookupError:pass
        return

assert os.environ.get('OUTER_TANK_NATIVE_GRANT')==PLAN['source_remote'], 'No explicit exact-cutoff serial grant'
assert free()>=START, 'Insufficient start margin; no metadata or compilation begun'
assert PLAN['remote_tree_leaves_verified']==len(PLAN['files'])==55
assert not SOURCE.exists() and not (OUT/'fixture-test-compile.claim').exists() and not (OUT/'rustc-invocations.jsonl').exists(), 'No automatic retries or overwritten evidence'
api=Path(PLAN['api_path']);api_prefix='pluto-lib/mixins/pluto_native_creative/inventory-api'
for p in PLAN['api_files']:assert h(api/Path(p).relative_to(api_prefix))==PLAN['files'][p]['sha256']
for p,row in PLAN['files'].items():
    if p in PLAN['api_files']:continue
    data=subprocess.check_output(['git','-C',str(REPO),'show',row['source_commit']+':'+p])
    assert hashlib.sha256(data).hexdigest()==row['sha256']
    dest=SOURCE/p;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data)
link=SOURCE/api_prefix;link.parent.mkdir(parents=True,exist_ok=True);link.symlink_to(api,target_is_directory=True)
assert link.resolve()==api.resolve()
assert source_seal()=={p:r['sha256'] for p,r in PLAN['files'].items()}
before=official();(OUT/'official-seal.json').write_text(json.dumps(before,sort_keys=True)+'\n')
(OUT/'cargo-tmp').mkdir();wrapper=OUT/'rustc_budget_wrapper.py';wrapper.chmod(0o755)
env=os.environ.copy()
for k in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER'):
    assert not env.get(k),(k,env.get(k))
env.update({'HORIZON_HOST':str(HOST),'CARGO_TARGET_DIR':str(BASE/'.work/official-advancement-recovery/host-target'),
    'CARGO_BUILD_JOBS':'1','CARGO_INCREMENTAL':'0','CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0',
    'CARGO_NET_OFFLINE':'true','GIT_OPTIONAL_LOCKS':'0','PYTHONDONTWRITEBYTECODE':'1','TMPDIR':str(OUT/'cargo-tmp'),'RUSTC_WRAPPER':str(wrapper)})
command=[sys.executable,str(SOURCE/'tconstruct/tests/run_outer_tank_patch.py')]
minimum=free();assert minimum>=START
start=time.monotonic();stopped=False;timed_out=False
with (OUT/'native.log').open('x') as log:
    process=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT,env=env,start_new_session=True)
    while process.poll() is None:
        minimum=min(minimum,free())
        if minimum<FLOOR or time.monotonic()-start>150:
            stopped=True;timed_out=time.monotonic()-start>150;stop(process);break
        time.sleep(.05)
    process.wait();minimum=min(minimum,free())
result={'source_remote':PLAN['source_remote'],'exit_code':process.returncode,'guard_stopped':stopped,'timed_out':timed_out,
    'minimum_free_bytes':minimum,'end_free_bytes':free(),'elapsed_seconds':time.monotonic()-start,'command':command,
    'source_seal_unchanged':source_seal()=={p:r['sha256'] for p,r in PLAN['files'].items()},'official_seal_unchanged':official()==before,
    'normal_floor_bytes':FLOOR,'required_start_margin_bytes':PLAN['required_start_margin']}
if (OUT/'rustc-invocations.jsonl').exists():
    calls=[json.loads(x) for x in (OUT/'rustc-invocations.jsonl').read_text().splitlines()];result['rustc_invocations']=calls
    result['fixture_compilations']=sum(x['decision']=='fixture_test_compilation' for x in calls)
    result['denied_calls']=sum(x['decision'].startswith('denied_') for x in calls)
summaries=re.findall(r'^test result: .*$',(OUT/'native.log').read_text(),re.M)
checks={
 'normal_terminal':not stopped and minimum>=FLOOR and not timed_out and process.returncode==0,
 'source_identity':result['source_seal_unchanged'] and result['official_seal_unchanged'],
 'only_one_fixture_compilation':result.get('fixture_compilations')==1 and result.get('denied_calls')==0,
 'exact_test_count':len(summaries)==1 and bool(re.fullmatch(r'test result: ok\. 160 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s',summaries[0])),
}
result['validation_checks']=checks;result['validation_passed']=all(checks.values());result['test_summaries']=summaries
(OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
assert result['validation_passed'],result
