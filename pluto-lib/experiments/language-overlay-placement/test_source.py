"""Synthetic schema/source checks only; no accepted native plan is implied."""
import ast
import copy
import importlib.util
import json
import os
from pathlib import Path
import tomllib
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('placement_source', HERE/'prepare.py')
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
if 'PLUTO_REPAIR_SOURCE' in os.environ: m.REPAIR = Path(os.environ['PLUTO_REPAIR_SOURCE'])
OFFICIAL = Path(os.environ['PLUTO_SELECTED_SOURCE'])
C = json.loads((HERE/'inputs.json').read_bytes())
NATIVE = json.loads((m.REPAIR/'inputs.json').read_bytes())['official_sources']


def synthetic():
    files = m.payload(OFFICIAL)
    manifest = tomllib.loads(files['horizon-mixin.toml'].decode())
    rows = []
    for item in manifest['injection']:
        row = {'mod_id':m.MOD,'mod_version':'0.0.0','injection_id':item['id'],
               'target':item['target'],'kind':item['kind'],'bindings':item['bindings'],
               'priority':100,'file':m.TARGET,'helper_source':files[item['helper']].decode(),
               'cancellable':item.get('cancellable',False),'node':[1,2], 'function_fingerprint':'f'*64}
        if item['kind'] == 'HEAD':
            row['contract'] = {'output':'std::ops::ControlFlow<Result<()>>','value':None}
        else:
            row.update(ordinal=0,require=1,variable={'at':'STORE','local':{'name':'parts','ordinal':0,'parameter':False,'mutable':True,'supported':True}},
                       variable_site=3,expression_site=3,site_fingerprint='e'*64,contract={'output':'Vec < String >','value':'Vec < String >'})
        rows.append(row)
    return files, {'schema':6,'transformer':'synthetic-only','inputs':NATIVE.copy(),'injections':rows,
                   'modules':[{'mod_id':m.MOD,'mod_version':'0.0.0','module_id':'bounded-merge','file':m.TARGET,
                               'name':'pluto_language_overlay','source':files['merge.rs'].decode(),'fingerprint':'d'*64}]}


class SourceTests(unittest.TestCase):
    def test_payload_is_exact_four_files_and_two_targets(self):
        files = m.payload(OFFICIAL)
        self.assertEqual(set(files), {'horizon-mixin.toml','merge.rs','write-head.rs','stamp-store.rs'})
        manifest = tomllib.loads(files['horizon-mixin.toml'].decode())
        self.assertEqual(len(manifest['module']),1); self.assertEqual(len(manifest['injection']),2)
        self.assertEqual(files['merge.rs'], (m.REPAIR/'merge.rs').read_bytes())

    def test_native_write_body_is_exact_small_inverse(self):
        native = (OFFICIAL/m.TARGET).read_bytes()
        self.assertEqual(m.expected_head(native), (HERE/'write-head.rs').read_bytes())
        self.assertEqual(len(m.native_write(native)), C['native_write_bytes'])
        self.assertEqual(m.sha(m.native_write(native)), C['native_write_sha256'])
        head = (HERE/'write-head.rs').read_bytes()
        self.assertNotIn(b'overlay.write(',head)
        self.assertLess(head.index(b'prepare(overlay.base'),head.index(b'overlay.dir(parent)?'))
        self.assertLess(head.index(b'fs::remove_file'),head.index(b'fs::write'))
        self.assertIn(b'ControlFlow::Continue(())',head)
        self.assertIn(b'ControlFlow::Break(result)',head)

    def test_no_broad_overwrite_or_direct_build_replacement(self):
        files = m.payload(OFFICIAL)
        manifest = tomllib.loads(files['horizon-mixin.toml'].decode())
        self.assertEqual([i['kind'] for i in manifest['injection']], ['HEAD','MODIFY_VARIABLE'])
        self.assertFalse(any(i['target'].endswith('::build') for i in manifest['injection']))

    def test_same_helper_fourteen_tests_and_official_serde_only(self):
        fixture = tomllib.loads((HERE/'Cargo.toml.in').read_text())
        host = tomllib.loads((OFFICIAL/'crates/horizon-client-lib/Cargo.toml').read_text())
        self.assertEqual(fixture['dependencies'], {k:host['dependencies'][k] for k in ['serde','serde_json']})
        self.assertEqual(len(C['expected_tests']),14); self.assertEqual(len(set(C['expected_tests'])),14)
        for path,digest in C['fixture_sha256'].items(): self.assertEqual(m.sha((HERE/path).read_bytes()),digest)
        self.assertEqual((HERE/'fixture.rs').read_text().count('mod pluto_language_overlay;'),1)

    def test_synthetic_inspector_acceptance_is_not_native_evidence(self):
        files, plan = synthetic(); result = m.inspect_plan(plan,files,NATIVE)
        self.assertEqual(result['transformer'],'synthetic-only')
        for key in ['native_apply','native_type_check','all_installed_composition','game_ui']:
            self.assertFalse(result[key])

    def test_unexpected_module_injection_or_other_state_rejects(self):
        for change in ['modules','injections','states','crates','bridges','data','post_build']:
            files,plan = synthetic()
            if change in ['modules','injections']: plan[change].append(copy.deepcopy(plan[change][0]))
            else: plan[change]=[{}]
            with self.assertRaises(ValueError): m.inspect_plan(plan,files,NATIVE)

    def test_native_source_mismatch_rejects(self):
        files,plan = synthetic(); plan['inputs'][m.TARGET]='0'*64
        with self.assertRaises(ValueError): m.inspect_plan(plan,files,NATIVE)
        with self.assertRaises(ValueError): m.native_write(b'not native')

    def test_wrong_target_binding_owner_or_payload_rejects(self):
        for key,value in [('target','wrong::write'),('bindings',[]),('mod_id','other'),('helper_source',''),('cancellable',False)]:
            files,plan=synthetic();plan['injections'][0][key]=value
            with self.assertRaises(ValueError):m.inspect_plan(plan,files,NATIVE)

    def test_store_selector_and_real_local_contract_are_required(self):
        for key,value in [('ordinal',1),('require',2),('variable_site',8),('site_fingerprint','bad')]:
            files,plan=synthetic();plan['injections'][1][key]=value
            with self.assertRaises(ValueError):m.inspect_plan(plan,files,NATIVE)
        for key,value in [('name','other'),('ordinal',1),('parameter',True),('mutable',False),('supported',False)]:
            files,plan=synthetic();plan['injections'][1]['variable']['local'][key]=value
            with self.assertRaises(ValueError):m.inspect_plan(plan,files,NATIVE)

    def test_wrong_return_or_store_value_contract_rejects(self):
        for at,key,value in [(0,'output','()'),(0,'value','u8'),(1,'output','String'),(1,'value','u8'),(1,'operation',{})]:
            files,plan=synthetic();plan['injections'][at]['contract'][key]=value
            with self.assertRaises(ValueError):m.inspect_plan(plan,files,NATIVE)

    def test_module_visibility_payload_and_schema_are_exact(self):
        for key,value in [('visibility','pub'),('source',''),('name','different')]:
            files,plan=synthetic();plan['modules'][0][key]=value
            with self.assertRaises(ValueError):m.inspect_plan(plan,files,NATIVE)
        files,plan=synthetic();plan['schema']=5
        with self.assertRaises(ValueError):m.inspect_plan(plan,files,NATIVE)

    def test_preparation_cannot_run_or_apply_tools(self):
        tree=ast.parse((HERE/'prepare.py').read_text())
        imports={n.module for n in ast.walk(tree) if isinstance(n,ast.ImportFrom)}
        imports|={a.name for n in ast.walk(tree) if isinstance(n,ast.Import) for a in n.names}
        self.assertFalse(imports&{'subprocess','shutil'})
        calls={n.func.attr for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute)}
        self.assertFalse(calls&{'write_bytes','write_text','mkdir','unlink','run','Popen'})


if __name__=='__main__':unittest.main()
