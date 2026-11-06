"""Bounded complete-real-API --lib proof; run only with the coordinator slot."""
from pathlib import Path
import hashlib,json,os,re,shutil,subprocess
ROOT=Path('/workspace/scratch/c7b540ff3393')
SOURCE=ROOT/'.work/pluto-menu-words-379'
OUT=ROOT/'.work/java-menu-foundation-proof-20261106'
API=SOURCE/'pluto-lib/mixins/pluto_native_creative/inventory-api'
STAGE=OUT/'api-unit-fixture/inventory-api'
SEED=SOURCE/'pluto-lib/tests/inventory-api.Cargo.lock'
plan=json.loads((OUT/'api-plan.json').read_text())
seal=json.loads((OUT/'source-seal.json').read_text())
def h(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def check_source():
    assert subprocess.check_output(['git','-C',str(SOURCE),'rev-parse','HEAD'],text=True).strip()==plan['source_commit']
    assert all(h(SOURCE/p)==value for p,value in seal['files'].items())
    assert all(h(API/p)==value for p,value in plan['api_files'].items())
    assert h(SEED)==plan['seed_lock_sha256']
check_source()
free=shutil.disk_usage(ROOT).free
assert free >= 10*1024**3+128*1024**2+16*1024**2, 'Need normal floor plus 16 MiB before starting'
for key,value in {'CARGO_INCREMENTAL':'0','CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_BUILD_JOBS':'1','CARGO_NET_OFFLINE':'true'}.items():
    assert os.environ.get(key)==value, (key,os.environ.get(key))
assert Path(os.environ['CARGO_TARGET_DIR']).resolve()==ROOT/'.work/official-advancement-recovery/host-target'
assert not os.environ.get('RUSTC_WORKSPACE_WRAPPER')
assert not os.environ.get('RUSTC_WRAPPER')
assert not os.environ.get('RUSTFLAGS') and not os.environ.get('CARGO_ENCODED_RUSTFLAGS')
assert not (OUT/'api-test-compile.claim').exists() and not (OUT/'rustc-invocations.jsonl').exists(), 'Refuse an unreviewed retry'
wrapper=OUT/'rustc_budget_wrapper.py'
wrapper.chmod(0o755)
os.environ['RUSTC_WRAPPER']=str(wrapper)
assert not STAGE.exists(), 'Refuse to overwrite a prior fixture'
shutil.copytree(API,STAGE)
shutil.copy2(SEED,STAGE/'Cargo.lock')
assert all(h(STAGE/p)==value for p,value in plan['api_files'].items())
assert h(STAGE/'Cargo.lock')==plan['seed_lock_sha256']
command=['cargo','test','--lib','--locked','--offline','--manifest-path',str(STAGE/'Cargo.toml')]
print('Real API source:',API,flush=True)
print('Exact temporary copy:',STAGE,flush=True)
print('Command:',command,flush=True)
def verify_success(returncode, output, invocations):
    if returncode != 0:
        return
    assert sum(row['decision']=='api_test_compilation' for row in invocations)==1, 'Expected exactly one API test compilation'
    assert all(not row['decision'].startswith('denied_') for row in invocations)
    summaries=re.findall(r'^test result: .*$', output, re.MULTILINE)
    assert len(summaries)==1 and re.fullmatch(r'test result: ok\. 125 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s', summaries[0]), summaries

result=None
checks={}
try:
    result=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    print(result.stdout,flush=True)
    invocations=[json.loads(line) for line in (OUT/'rustc-invocations.jsonl').read_text().splitlines()]
    verify_success(result.returncode,result.stdout,invocations)
finally:
    try:
        check_source()
        assert all(h(STAGE/p)==value for p,value in plan['api_files'].items())
        assert h(STAGE/'Cargo.lock')==plan['seed_lock_sha256']
        checks={'all_materialized_source_hashes_before_after':True,'seed_lock_before_after':True,'copy_unchanged':True}
    finally:
        (OUT/'api-unit-source-check.json').write_text(json.dumps({'exit_code':None if result is None else result.returncode,'source':plan['source_commit'],'real_source':str(API),'unchanged_copy':str(STAGE),'api_file_count':len(plan['api_files']),'checks':checks,'command':command,'profile':{k:os.environ.get(k) for k in ['CARGO_TARGET_DIR','CARGO_INCREMENTAL','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_BUILD_JOBS','RUSTFLAGS','RUSTC_WRAPPER']}},indent=2)+'\n')
raise SystemExit(result.returncode)
