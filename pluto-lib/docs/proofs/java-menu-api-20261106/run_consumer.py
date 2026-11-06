"""Run only the specifically granted java OR menu proof. Never starts both."""
from pathlib import Path
import hashlib,json,os,re,runpy,shutil,subprocess,sys
ROOT=Path('/workspace/scratch/c7b540ff3393')
OUT=ROOT/'.work/java-menu-foundation-proof-20261106'
SOURCE=ROOT/'.work/pluto-menu-words-379'
API=SOURCE/'pluto-lib/mixins/pluto_native_creative/inventory-api'
config=runpy.run_path(str(OUT/'rustc_consumer_wrapper.py'),run_name='not_main')['CONFIG']
assert len(sys.argv)==2 and sys.argv[1] in config
family=sys.argv[1];name,manifest,entry,count=config[family]
plan=json.loads((OUT/'api-plan.json').read_text());seal=json.loads((OUT/'source-seal.json').read_text())
def h(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def check():
 assert subprocess.check_output(['git','-C',str(SOURCE),'rev-parse','HEAD'],text=True).strip()==plan['source_commit']
 assert all(h(SOURCE/p)==sha for p,sha in seal['files'].items())
 assert all(h(API/p)==sha for p,sha in plan['api_files'].items())
 assert h(SOURCE/'pluto-lib/tests/inventory-api.Cargo.lock')==plan['seed_lock_sha256']
check()
assert shutil.disk_usage(ROOT).free>=10*1024**3+128*1024**2+14*1024**2
for key,value in {'CARGO_INCREMENTAL':'0','CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_BUILD_JOBS':'1','CARGO_NET_OFFLINE':'true','PYTHONDONTWRITEBYTECODE':'1'}.items():assert os.environ.get(key)==value,(key,os.environ.get(key))
assert Path(os.environ['CARGO_TARGET_DIR']).resolve()==ROOT/'.work/official-advancement-recovery/host-target'
assert not any(os.environ.get(k) for k in ['RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS'])
assert not (OUT/(family+'-rustc-invocations.jsonl')).exists()
assert not list(OUT.glob(family+'-*.claim'))
wrapper=OUT/'rustc_consumer_wrapper.py';wrapper.chmod(0o755)
os.environ['RUSTC_WRAPPER']=str(wrapper);os.environ['PLUTO_PROOF_CONSUMER']=family
command=['cargo','test','--lib','--locked','--offline','--manifest-path',str(manifest/'Cargo.toml')]
result=None;strict=False;unchanged=False
try:
 result=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 print(result.stdout,flush=True)
 rows=[json.loads(s) for s in (OUT/(family+'-rustc-invocations.jsonl')).read_text().splitlines()]
 if result.returncode==0:
  assert sum(r['decision']=='api_library_compilation' for r in rows)<=1
  assert sum(r['decision']=='consumer_test_compilation' for r in rows)==1
  assert not any(r['decision'].startswith('denied_') for r in rows)
  summaries=re.findall(r'^test result: .*$',result.stdout,re.MULTILINE)
  assert len(summaries)==1 and re.fullmatch(r'test result: ok\. '+str(count)+r' passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s',summaries[0]),summaries
  strict=True
finally:
 try:check();unchanged=True
 finally:(OUT/(family+'-source-check.json')).write_text(json.dumps({'family':family,'source':plan['source_commit'],'cargo_exit_code':None if result is None else result.returncode,'strict_success':strict,'source_api_seed_seals_unchanged':unchanged,'real_manifest_path':str(manifest/'Cargo.toml'),'real_api_path':str(API),'command':command},indent=2)+'\n')
raise SystemExit(result.returncode)
