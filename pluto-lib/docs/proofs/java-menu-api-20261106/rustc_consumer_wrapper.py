#!/usr/bin/env python3
"""One explicitly selected real consumer harness, at most one real API library."""
import json,os,runpy,sys
from pathlib import Path
OUT=Path(__file__).resolve().parent
SOURCE=OUT.parent/'pluto-menu-words-379'
API=SOURCE/'pluto-lib/mixins/pluto_native_creative/inventory-api'
TARGET=OUT.parent/'official-advancement-recovery/host-target/debug/deps'
BASE=runpy.run_path(str(OUT/'rustc_budget_wrapper.py'),run_name='not_main')
values=BASE['values']
CONFIG={
 'java':('pluto_java_hashmap_tests',SOURCE/'pluto-lib/tests/java-hashmap','src/lib.rs',8),
 'menu':('tconstruct_builder_wire_tests',SOURCE/'tconstruct/tests/builder-wire','../rust/source_builder_wire.rs',11),
}

def classify(args,cwd,manifest,family):
 query=BASE['classify'](args,cwd,manifest)
 if query in {'version_query','print_query'}:return query
 if family not in CONFIG:return 'denied_family'
 name,root,source,_=CONFIG[family]
 crate=values(args,'--crate-name')
 if crate==['pluto_inventory_api']:
  if '--test' in args or values(args,'--crate-type')!=['lib'] or values(args,'--emit')!=['dep-info,metadata,link']:
   return 'denied_api_mode'
  expected_root=API;expected_source=API/'src/lib.rs';result='api_library_compilation'
 elif crate==[name]:
  if args.count('--test')!=1 or values(args,'--crate-type') or values(args,'--emit')!=['dep-info,link']:
   return 'denied_fixture_mode'
  expected_root=root;expected_source=(root/source).resolve();result='consumer_test_compilation'
 else:return 'denied_other_compilation'
 if Path(cwd).resolve()!=expected_root or not manifest or Path(manifest).resolve()!=expected_root:return 'denied_package_path'
 sources=[a for a in args if a.endswith('.rs')]
 if len(sources)!=1 or (Path(cwd)/sources[0]).resolve()!=expected_source:return 'denied_source_path'
 if values(args,'--out-dir')!=[str(TARGET)] or any('incremental=' in a for a in args):return 'denied_output'
 return result

def main():
 compiler,args=sys.argv[1],sys.argv[2:]
 family=os.environ.get('PLUTO_PROOF_CONSUMER','')
 decision=classify(args,os.getcwd(),os.environ.get('CARGO_MANIFEST_DIR'),family)
 if decision in {'api_library_compilation','consumer_test_compilation'}:
  try:
   fd=os.open(OUT/(family+'-'+decision+'.claim'),os.O_CREAT|os.O_EXCL|os.O_WRONLY,0o600);os.close(fd)
  except FileExistsError:decision='denied_second_compilation'
 with (OUT/(family+'-rustc-invocations.jsonl')).open('a') as f:
  f.write(json.dumps({'compiler':compiler,'args':args,'cwd':os.getcwd(),'manifest':os.environ.get('CARGO_MANIFEST_DIR'),'decision':decision})+'\n')
 if decision not in {'version_query','print_query','api_library_compilation','consumer_test_compilation'}:
  print('Refused rustc before execution: '+decision,file=sys.stderr);return 91
 os.execv(compiler,[compiler,*args])
if __name__=='__main__':raise SystemExit(main())
