"""Replay all12 unchanged guards against one downloaded, byte-fixed complete plan."""
import importlib.util,json,os,re,sys
from pathlib import Path
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
def load(name,path):
 s=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m
prior=load('reviewed_plan_runner',HERE.parent/'playtest-plan-ci/run.py')
common=prior.common;package=prior.package;evidence=prior.evidence;Runner=prior.Runner;sha=common.sha
BRANCH='checkpoint/tinkers-playtest-next-guard-replay-ci-20261107'
HEAD=None
TREE=None  # Final metadata-only integration cutoff requires review before admission.
TESTS=32
OWN={'pluto-lib/experiments/playtest-guard-replay-ci/'+n for n in ('run.py','test_runner.py','README.md')}|{'.github/workflows/playtest-guard-replay-wip.yml'}
REQUIRED={'runner','artifact','fixed_checkouts','official_inputs','package_headers','package_equality','full_plan_contract','formatter','crafting_package','complete_guards'}
def inputs():return json.loads((HERE/'inputs.json').read_text())
def validate(c):
 if c.get('ready') is not True or not HEAD or not TREE:raise ValueError('Replay cutoff is not ready')
 if (c['integration']['head'],c['integration']['tree'],c['branch'])!=(HEAD,TREE,BRANCH):raise ValueError('Wrong replay cutoff')
 if set(c['source_sha256'])!=OWN:raise ValueError('Replay source set changed')
 for name,digest in c['source_sha256'].items():
  if sha(package.regular(ROOT/name,2*1024**2))!=digest:raise ValueError('Changed replay controller')
 prior.validate(prior.inputs())
def event_allowed(env,event):
 return env.get('GITHUB_ACTIONS')=='true' and env.get('GITHUB_REPOSITORY')==prior.REPOSITORY and env.get('GITHUB_EVENT_NAME')=='push' and env.get('GITHUB_REF')=='refs/heads/'+BRANCH and event.get('repository',{}).get('private') is False
def setup_guard(report):
 c=inputs();validate(c)
 if not event_allowed(os.environ,json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())):raise ValueError('Exact public CI only')
 if common.git(ROOT,'rev-parse','HEAD')!=os.environ['GITHUB_SHA']:raise ValueError('Wrong replay head')
 common.assert_git_clean(ROOT)
 work=Path(os.environ['RUNNER_TEMP'])/('guard-replay-'+os.environ['GITHUB_SHA']);env=os.environ.copy()
 for k in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO','RUSTC','GIT_DIR','GIT_WORK_TREE','GIT_INDEX_FILE'):
  env.pop(k,None)
 env.update(CARGO_HOME=str(work/'cargo-home'),CARGO_NET_OFFLINE='true',RUSTUP_TOOLCHAIN='1.95.0',PYTHONDONTWRITEBYTECODE='1',GIT_OPTIONAL_LOCKS='0')
 runner=Runner(work,env,report);runner.guard(reserve=1024**3)
 return c,work,env,runner
def artifact(directory,c):
 expected=c['artifact']['files'];directory=Path(directory)
 if {p.name for p in directory.iterdir()}!=set(expected):raise ValueError('Downloaded artifact file set changed')
 evidence.verify_saved(directory)
 for name,want in expected.items():
  raw=evidence.read_regular(directory/name,evidence.LIMITS[name])
  if (len(raw),sha(raw))!=(want['bytes'],want['sha256']):raise ValueError('Downloaded evidence bytes differ')
 receipt=json.loads((directory/'receipt.json').read_text());plan=json.loads((directory/'plan.json').read_text())
 if receipt['head']!=c['artifact']['source_head'] or not all(receipt[k] is True for k in ('source_verified','diagnostic_complete','probe_completed','plan_accepted')):raise ValueError('Wrong original accepted evidence')
 if receipt['gates']['fixed_checkouts']['integration']['head']!=c['artifact']['original_integration']:raise ValueError('Wrong original integration')
 return receipt,plan
def verify_inputs(plan,actual):
 wanted=plan.get('inputs')
 if not isinstance(wanted,dict) or not wanted or any(not isinstance(k,str) or Path(k).is_absolute() or '..' in Path(k).parts or not isinstance(v,str) or not re.fullmatch('[0-9a-f]{64}',v) for k,v in wanted.items()):raise ValueError('Invalid plan input map')
 if actual!=wanted:raise ValueError('Whole official input file set/hash differs')
 return {'passed':True,'count':len(actual),'sha256':sha(json.dumps(actual,sort_keys=True).encode())}
def same_package(actual,old):
 expected={k:v for k,v in old.items() if k!='passed'}
 if actual!=expected:raise ValueError('Repackaged header/member/archive differs from original plan evidence')
SUPPORT='mixins/tconstruct_native/data/tconstruct-support.zip'
COMBINED_SUPPORT=SUPPORT.removeprefix('mixins/')
def native_package_equality(actual,old,plan):
 # One explicitly separated non-code data entry. Never claim archive/data equivalence.
 previous={k:v for k,v in old.items() if k!='passed'}
 if set(actual)!=set(previous):raise ValueError('TConstruct package evidence schema changed')
 for key in ('id','members','header','file_mode'):
  if actual[key]!=previous[key]:raise ValueError('TConstruct header/member count changed')
 before=previous['member_sha256'];after=actual['member_sha256']
 if set(before)!=set(after) or SUPPORT not in before:raise ValueError('TConstruct member set changed')
 if {k:v for k,v in before.items() if k!=SUPPORT}!={k:v for k,v in after.items() if k!=SUPPORT}:raise ValueError('TConstruct native member drift')
 if plan.get('data')!=[{'mod_id':'tconstruct_native','files':{'tconstruct-support.zip':before[SUPPORT]}}]:raise ValueError('Original plan data identity differs')
 return {'passed':True,'native_members_equal':True,'headers_equal':True,'archive_equal':actual['archive_sha256']==previous['archive_sha256'],
  'support_changed':after[SUPPORT]!=before[SUPPORT], 'support_before_sha256':before[SUPPORT],'support_after_sha256':after[SUPPORT],
  'archive_before_sha256':previous['archive_sha256'],'archive_after_sha256':actual['archive_sha256'],
  'archive_before_bytes':previous['archive_bytes'],'archive_after_bytes':actual['archive_bytes'],
  'support_before':previous['support'],'support_after':actual['support'],
  'support_before_map_sha256':previous['support_seal_sha256'],'support_after_map_sha256':actual['support_seal_sha256'],
  'scope':'Only native members and headers are equal. Current support data is independently verified against the fixed integration checkout; old plan data is not replaced. No apply or full archive equivalence.'}
def native_combined_equality(actual,old):
 previous={k:v for k,v in old.items() if k!='passed'}
 if set(actual)!=set(previous) or actual['headers']!=previous['headers'] or actual['members']!=previous['members']:raise ValueError('Combined evidence/header/count differs')
 before=previous['member_sha256'];after=actual['member_sha256']
 if set(before)!=set(after) or COMBINED_SUPPORT not in before:raise ValueError('Combined member set differs')
 if {k:v for k,v in before.items() if k!=COMBINED_SUPPORT}!={k:v for k,v in after.items() if k!=COMBINED_SUPPORT}:raise ValueError('Combined native member drift')
 return {'passed':True,'native_members':len(after)-1,'native_map_sha256':sha(json.dumps({k:v for k,v in after.items() if k!=COMBINED_SUPPORT},sort_keys=True).encode()),'old_complete_map_sha256':previous['members_seal_sha256'],'current_complete_map_sha256':actual['members_seal_sha256']}
def forbidden_tools(directory):
 directory.mkdir()
 for name in ('cargo','rustc'):
  p=directory/name;p.write_text('#!/usr/bin/env python3\nimport sys\nsys.stderr.write("Guard-only: compiler/metadata disabled\\n")\nsys.exit(97)\n');p.chmod(0o755)
def execute(report):
 c,work,env,runner=setup_guard(report);original=common.checkout_seal(ROOT);git_original=package.git_state(ROOT)
 source=Path(os.environ['RUNNER_TEMP'])/'guard-plan-input';snapshots={};git_snapshots={}
 try:
  work.mkdir();Path(env['CARGO_HOME']).mkdir()
  old,plan=artifact(source,c);snapshots[str(source)]=common.checkout_seal(source)
  report['gates']['artifact']={'passed':True,**c['artifact']}
  output=runner.run('runner-checks',[sys.executable,str(HERE/'test_runner.py'),'-v']);common.validate_python_checks(output,TESTS);report['gates']['runner']={'passed':True,'tests':TESTS}
  roots={}
  for name,url,rev in [('horizon','https://github.com/horizon-rust/horizon.git',package.HORIZON),('integration','https://github.com/sofia-gros/horizon-mods.git',HEAD)]:
   root=work/(name+'-git');roots[name]=root
   for suffix,args in [('init',['git','init',str(root)]),('origin',['git','-C',str(root),'remote','add','origin',url]),('fetch',['git','-C',str(root),'fetch','--depth','1','origin',rev]),('checkout',['git','-C',str(root),'checkout','--detach','FETCH_HEAD'])]:runner.run(name+'-'+suffix,args,timeout=600)
   if common.git(root,'rev-parse','HEAD')!=rev or common.git(root,'remote','get-url','origin')!=url:raise ValueError('Wrong fixed checkout')
   common.assert_git_clean(root);snapshots[str(root)]=common.checkout_seal(root);git_snapshots[str(root)]=package.git_state(root)
  integration=roots['integration']
  if common.git(integration,'rev-parse','HEAD^{tree}')!=TREE:raise ValueError('Wrong integration tree')
  for path,digest in c['integration']['source_sha256'].items():
   if sha(package.regular(integration/path,2*1024**2))!=digest:raise ValueError('Changed fixed guard/metadata input')
  if sha((roots['horizon']/'Cargo.lock').read_bytes())!=prior.OFFICIAL['horizon']['lock']:raise ValueError('Wrong official lock')
  report['gates']['fixed_checkouts']={'passed':True,'integration':c['integration'],'horizon':prior.OFFICIAL['horizon']}
  official=work/'horizon';runner.run('official-export',[sys.executable,str(HERE/'run.py'),'--export',str(roots['horizon']),package.HORIZON,str(official)])
  snapshots[str(official)]=common.checkout_seal(official)
  report['gates']['official_inputs']=verify_inputs(plan,common.seal(official))
  report['gates']['package_headers']={'passed':True,**package.validate_headers(integration)}
  tc=work/'tconstruct.hnmod';pluto=work/'plutolib.hnmod'
  runner.run('tconstruct-package',[sys.executable,str(integration/'tconstruct/tools/package_hnmod.py'),'--output',str(tc)],cwd=integration)
  tc_value=package.verify_tconstruct(integration,tc);tc_value['support'].pop('member_sha256');tc_equality=native_package_equality(tc_value,old['gates']['tconstruct_bundle'],plan)
  runner.run('pluto-package',[sys.executable,str(integration/'pluto-lib/tools/package_native.py'),str(pluto)],cwd=integration)
  pluto_value=package.verify_pluto(integration,pluto);same_package(pluto_value,old['gates']['pluto_bundle'])
  mods=work/'combined-mods';combined=package.compose(tc,pluto,mods);combined_equality=native_combined_equality(combined,old['gates']['combined_payload']);snapshots[str(mods)]=common.checkout_seal(mods)
  report['gates']['package_equality']={'passed':True,'tconstruct_archive_sha256':tc_value['archive_sha256'],'pluto_archive_sha256':pluto_value['archive_sha256'],'combined_members':combined['members'],'combined_map_sha256':combined['members_seal_sha256'],'tconstruct':tc_equality,'combined':combined_equality,'scope':'Native guard replay only; support data difference is not apply admission'}
  report['gates']['full_plan_contract']=package.verify_plan(source/'plan.json',mods,plan['inputs'])
  runner.run('formatter-install',['rustup','toolchain','install','1.95.0','--profile','minimal','--component','rustfmt'],timeout=600)
  formatter=Path(runner.run('formatter-path',['rustup','which','--toolchain','1.95.0','rustfmt']).strip()).resolve()
  toolchains=Path(env.get('RUSTUP_HOME',str(Path.home()/'.rustup'))).resolve()/'toolchains'
  if not formatter.is_relative_to(toolchains):raise ValueError('Unexpected rustfmt identity')
  report['gates']['formatter']={'passed':True,'path':str(formatter),'sha256':sha(formatter.read_bytes()),'version':runner.run('formatter-version',[str(formatter),'--version']).strip(),'scope':'Existing Linux1.95 guard normalization only; not latest Windows formatter proof'}
  wrappers=work/'no-compile';forbidden_tools(wrappers);snapshots[str(wrappers)]=common.checkout_seal(wrappers)
  env.update(CARGO=str(wrappers/'cargo'),RUSTC=str(wrappers/'rustc'),PATH=str(wrappers)+os.pathsep+str(formatter.parent)+os.pathsep+env['PATH'])
  contract_path=work/'crafting-package-contract.json'
  expected_archives={'tconstruct_native':tc_value['archive_sha256'],'pluto_native_creative':pluto_value['archive_sha256']}
  contract=prior.write_package_contract(integration,{'tconstruct_native':tc,'pluto_native_creative':pluto},expected_archives,contract_path)
  report['gates']['crafting_package']=contract
  archive_seals={tc:expected_archives['tconstruct_native'],pluto:expected_archives['pluto_native_creative']}
  guards=prior.attempt_guards(runner,source/'plan.json',official,integration,contract_path);report['gates']['complete_guards']=guards
  report['compatibility']=False if not guards['passed'] else None
  if set(report['gates'])!=REQUIRED:raise ValueError('Missing replay gate')
  report['diagnostic_complete']=True
 except BaseException as exc:common.record_error(report,exc)
 finally:
  try:
   common.assert_git_clean(ROOT);common.assert_full_seal(ROOT,original)
   if package.git_state(ROOT)!=git_original:raise ValueError('Controller Git state drift')
   for p,v in snapshots.items():common.assert_full_seal(Path(p),v)
   for p,v in git_snapshots.items():
    common.assert_git_clean(Path(p))
    if package.git_state(Path(p))!=v:raise ValueError('Checkout Git state drift')
   if 'crafting_package' in report['gates']:
    if sha(package.regular(contract_path,prior.guard_once.MAX_CONTRACT))!=report['gates']['crafting_package']['sha256']:raise ValueError('Crafting contract changed')
    for p,digest in archive_seals.items():
     if sha(package.regular(p,package.MAX_ARCHIVE))!=digest:raise ValueError('Admitted package changed')
   if 'formatter' in report['gates'] and sha(Path(report['gates']['formatter']['path']).read_bytes())!=report['gates']['formatter']['sha256']:raise ValueError('Formatter changed')
   runner.guard();report['source_verified']=True
  except BaseException as exc:common.record_error(report,exc,'final verification: ')
  report.update(minimum_free_bytes=runner.minimum_free,minimum_memory_bytes=getattr(runner,'minimum_memory',None),input_seals={k:sha(json.dumps(v,sort_keys=True).encode()) for k,v in snapshots.items()},controller_seal_sha256=sha(json.dumps(original,sort_keys=True).encode()))
def main():
 report={'head':os.environ.get('GITHUB_SHA'),'gates':{},'phases':[],'error':None,'source_verified':False,'diagnostic_complete':False,'probe_completed':False,'compatibility':None,'plan_regenerated':False,'apply_executed':False,'native_type_verified':False}
 try:execute(report)
 except BaseException as exc:common.record_error(report,exc)
 report['probe_completed']=report['diagnostic_complete'] and report['source_verified'] and report['error'] is None
 print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
 if not report['probe_completed']:raise SystemExit(1)
if __name__=='__main__':
 if sys.argv[1:]==['--preflight']:
  setup_guard({'phases':[]});target=Path(os.environ['RUNNER_TEMP'])/'guard-plan-input'
  if target.exists() or target.is_symlink():raise SystemExit('Artifact destination is not fresh')
 elif len(sys.argv)==5 and sys.argv[1]=='--export':prior.export_latest(Path(sys.argv[2]),sys.argv[3],Path(sys.argv[4]))
 elif len(sys.argv)==1:main()
 else:raise SystemExit('Invalid replay arguments')
