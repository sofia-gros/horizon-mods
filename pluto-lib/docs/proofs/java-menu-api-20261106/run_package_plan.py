"""One archive-backed plan and mandatory preflight; no apply, Cargo or host copy."""
from pathlib import Path
import hashlib,json,os,stat,subprocess,tarfile,zipfile
ROOT=Path('/workspace/scratch/c7b540ff3393')
OUT=ROOT/'.work/java-menu-foundation-proof-20261106'
SOURCE=ROOT/'.work/pluto-menu-words-379'
CENTRAL=ROOT/'upgrade-official-baseline-20261105'
HOST=ROOT/'.devtools/latest-official-export-20261105/horizon'
CLI=ROOT/'.work/official-advancement-recovery/cli-target/release/horizon-mixins'
ARCHIVE=OUT/'plutolib.hnmod'
PACKAGED=OUT/'packaged'
PLAN=OUT/'package-plan.json'
RUNTIME='69c6b72b0b79b64317483bede6a2b2a689d5b83a'
def h(p):return hashlib.sha256(p.read_bytes()).hexdigest()
seal=json.loads((OUT/'source-seal.json').read_text())['files']
with tarfile.open(CENTRAL/'.work/plutolib-foundation-proof/grouped-v15-source-plan/pluto-v15-source-plan-evidence.tar.gz') as t:
 prior=json.load(t.extractfile(next(m for m in t.getmembers() if m.name.endswith('/plan.json'))))
inputs=prior['inputs']
# The reviewed dispatcher imports these scripts/metadata and native root helpers.
guard_files={}
for directory,patterns in [(CENTRAL/'pluto-lib/tools',['*.py','*.json']),(CENTRAL/'pluto-lib/mixins/pluto_native_creative',['*.rs'])]:
 for pattern in patterns:
  for p in directory.glob(pattern):
   relative=p.relative_to(CENTRAL).as_posix()
   expected=subprocess.check_output(['git','-C',str(SOURCE),'show',RUNTIME+':'+relative])
   assert p.read_bytes()==expected,relative
   guard_files[relative]=h(p)
def verify():
 assert subprocess.check_output(['git','-C',str(SOURCE),'rev-parse','HEAD'],text=True).strip()==RUNTIME
 assert all(h(SOURCE/p)==sha for p,sha in seal.items())
 assert h(ARCHIVE)=='5adf8885829c7665a01f477099cb0e0125362e1bc12cf20d1536726e999b09ce'
 assert h(CLI)=='50fd3105daafd09525889c11c2374a727dbd41b0bad0a5b8714c01b0d06866ff'
 assert all(h(HOST/p)==sha for p,sha in inputs.items())
 assert all(h(CENTRAL/p)==sha for p,sha in guard_files.items())
verify()
assert not PLAN.exists() and not PACKAGED.exists(), 'Refuse unreviewed retry'
assert os.environ.get('PYTHONDONTWRITEBYTECODE')=='1'
with zipfile.ZipFile(ARCHIVE) as z:
 entries=z.infolist();assert len(entries)==141 and sum(i.file_size for i in entries)==678967
 for i in entries:
  p=Path(i.filename)
  assert not p.is_absolute() and '..' not in p.parts and not stat.S_ISLNK(i.external_attr>>16)
 z.extractall(PACKAGED)
 extracted={p.relative_to(PACKAGED).as_posix():h(p) for p in PACKAGED.rglob('*') if p.is_file()}
commands=[
 [str(CLI),'plan','--workspace',str(HOST),'--mods',str(PACKAGED/'mixins'),'--out',str(PLAN)],
 ['python',str(CENTRAL/'pluto-lib/tools/check_native_plan.py'),str(PLAN),str(HOST)],
]
groups=['protection','advancement','damage_equipment','client_payload','recipe_placement','current_book','wire_snapshot','crafting_match','terrain_binding','cloud_insertion','station_ingredient','terrain_cell']
commands.extend(['python',str(CENTRAL/'pluto-lib/tools'/('check_'+group+'_plan.py')),str(PLAN)]+([] if group=='protection' else [str(HOST)]) for group in groups)
results=[];verified=False
try:
 for command in commands:
  print('Command:',command,flush=True)
  r=subprocess.run(command);results.append({'command':command,'exit_code':r.returncode})
  if r.returncode:raise SystemExit(r.returncode)
 plan=json.loads(PLAN.read_text())
 assert plan['inputs']==inputs
 counts={'crates':len(plan['crates']),'modules':len(plan['modules']),'injections':len(plan['injections']),'states':len(plan['states'])}
 assert counts=={'crates':2,'modules':66,'injections':34,'states':3}
 api=next(c for c in plan['crates'] if c['package']=='pluto-inventory-api')
 for p,sha in json.loads((OUT/'api-plan.json').read_text())['api_files'].items():assert api['files'][p]==sha,(p,api['files'].get(p))
 verified=True
finally:
 verify()
 assert all(h(PACKAGED/p)==sha for p,sha in extracted.items())
 result={'source':RUNTIME,'archive_sha256':h(ARCHIVE),'cli_sha256':h(CLI),'official_horizon':'624c4f6265c02296fbe8a80f029dd0d987fb94c0','official_mixins':'e8cd0aa38cffef49cb2e088ef716baa107148c02','phases':results,'package_plan_and_12_guards_passed':verified,'official_inputs_before_after':len(inputs),'source_before_after':len(seal),'guard_sources_exact_runtime_before_after':guard_files,'extracted_sources_before_after':len(extracted),'plan_sha256':h(PLAN) if PLAN.exists() else None,'plan_bytes':PLAN.stat().st_size if PLAN.exists() else None,'apply_performed':False,'host_typecheck_performed':False}
 (OUT/'package-plan-receipt.json').write_text(json.dumps(result,indent=2)+'\n')
