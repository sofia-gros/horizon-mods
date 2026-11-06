"""Exact published presentation API134 proof. No native adapter build."""
from pathlib import Path
import hashlib,json,os,re,shutil,stat,subprocess
ROOT=Path('/workspace/scratch/c7b540ff3393')
OUT=ROOT/'.work/recipe-presentation-proof-20261106'
BASE=ROOT/'.work/pluto-menu-words-379'
PATCH=ROOT/'.work/recipe-presentation-source-20261106'
API=BASE/'pluto-lib/mixins/pluto_native_creative/inventory-api'
OVERLAY=PATCH/'pluto-lib/mixins/pluto_native_creative/inventory-api'
SEED=BASE/'pluto-lib/tests/inventory-api.Cargo.lock'
STAGE=OUT/'api-unit-fixture/inventory-api'
plan=json.loads((OUT/'api-plan.json').read_text())
base_seal=json.loads((ROOT/'.work/java-menu-foundation-proof-20261106/source-seal.json').read_text())['files']
patch_seal=json.loads((PATCH/'pluto-lib/docs/recipe-book-presentation-validation-20261106.json').read_text())['source_files_sha256']
def h(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def exact_tree(root, expected):
 assert not root.is_symlink() and root.is_dir(), str(root)
 expected=set(expected)
 allowed_dirs={str(parent) for path in expected for parent in Path(path).parents if str(parent)!='.'}
 actual=set()
 for parent, dirs, files in os.walk(root, followlinks=False):
  for name in dirs:
   path=Path(parent)/name; relative=path.relative_to(root).as_posix()
   assert stat.S_ISDIR(path.lstat().st_mode) and relative in allowed_dirs, ('unexpected directory or symlink',relative)
  for name in files:
   path=Path(parent)/name; relative=path.relative_to(root).as_posix()
   assert stat.S_ISREG(path.lstat().st_mode), ('not regular',relative)
   actual.add(relative)
 assert actual==expected, ('file-set mismatch',sorted(actual-expected),sorted(expected-actual))
 return actual
api_prefix='pluto-lib/mixins/pluto_native_creative/inventory-api/'
base_api={p[len(api_prefix):]:sha for p,sha in base_seal.items() if p.startswith(api_prefix)}
assert len(base_api)==30
stage_files=set(plan['api_files'])|{'Cargo.lock'}
def source_for(path):return OVERLAY/path if path in plan['overlay_files'] else API/path
def check():
 exact_tree(API,base_api)
 exact_tree(OVERLAY,plan['overlay_files'])
 assert subprocess.check_output(['git','-C',str(BASE),'rev-parse','HEAD'],text=True).strip()==plan['base_runtime']
 assert all(h(BASE/p)==sha for p,sha in base_seal.items())
 assert all(h(PATCH/p)==sha for p,sha in patch_seal.items())
 assert all(h(source_for(p))==sha for p,sha in plan['api_files'].items())
 assert h(SEED)==plan['seed_lock_sha256']
check()
assert shutil.disk_usage(ROOT).free>=10*1024**3+128*1024**2+12*1024**2
for key,value in {'CARGO_INCREMENTAL':'0','CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_BUILD_JOBS':'1','CARGO_NET_OFFLINE':'true','PYTHONDONTWRITEBYTECODE':'1'}.items():assert os.environ.get(key)==value,(key,os.environ.get(key))
assert Path(os.environ['CARGO_TARGET_DIR']).resolve()==ROOT/'.work/official-advancement-recovery/host-target'
assert not any(os.environ.get(k) for k in ['RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS'])
assert not STAGE.exists() and not (OUT/'api-test-compile.claim').exists() and not (OUT/'rustc-invocations.jsonl').exists()
shutil.copytree(API,STAGE)
for path in plan['overlay_files']:shutil.copyfile(OVERLAY/path,STAGE/path)
shutil.copyfile(SEED,STAGE/'Cargo.lock')
assert len(plan['api_files'])==31
exact_tree(STAGE,stage_files)
assert all(h(STAGE/p)==sha for p,sha in plan['api_files'].items())
wrapper=OUT/'rustc_budget_wrapper.py';wrapper.chmod(0o755);os.environ['RUSTC_WRAPPER']=str(wrapper)
command=['cargo','test','--lib','--locked','--offline','--manifest-path',str(STAGE/'Cargo.toml')]
result=None;strict=False;unchanged=False
try:
 result=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 print(result.stdout,flush=True)
 rows=[json.loads(s) for s in (OUT/'rustc-invocations.jsonl').read_text().splitlines()]
 if result.returncode==0:
  assert sum(r['decision']=='api_test_compilation' for r in rows)==1
  assert not any(r['decision'].startswith('denied_') for r in rows)
  summaries=re.findall(r'^test result: .*$',result.stdout,re.MULTILINE)
  assert len(summaries)==1 and re.fullmatch(r'test result: ok\. 134 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s',summaries[0]),summaries
  strict=True
finally:
 try:
  check()
  exact_tree(STAGE,stage_files)
  assert all(h(STAGE/p)==sha for p,sha in plan['api_files'].items())
  assert h(STAGE/'Cargo.lock')==plan['seed_lock_sha256']
  unchanged=True
 finally:
  (OUT/'api-unit-source-check.json').write_text(json.dumps({'source':plan['remote_source'],'tree':plan['remote_tree'],'cargo_exit_code':None if result is None else result.returncode,'strict_success':strict,'base_patch_api_copy_seed_unchanged':unchanged,'exact_base_overlay_copy_file_sets':unchanged,'api_files':31,'base_materialized_files':len(base_seal),'patch_source_files':len(patch_seal),'actual_api_path':str(STAGE),'command':command},indent=2)+'\n')
raise SystemExit(result.returncode)
