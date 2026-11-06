"""Source/packaging and conservative composition checks, not GPU execution."""
import copy
import importlib.util
import json
from pathlib import Path
import tomllib
import unittest
ROOT=Path(__file__).resolve().parents[1];NATIVE=ROOT/'mixins/pluto_native_creative'
spec=importlib.util.spec_from_file_location('terrain_cell_guard',ROOT/'tools/check_terrain_cell_plan.py');guard=importlib.util.module_from_spec(spec);spec.loader.exec_module(guard)
class TerrainCellChecks(unittest.TestCase):
    def plan(self):
        m=guard.metadata()
        return {'schema':6,'inputs':m['source_sha256'],'injections':[{'mod_id':'pluto_native_creative','injection_id':m['injection_id'],**{k:m[k] for k in ['target','function_fingerprint','bindings','priority','kind','cancellable']},**m['location'],'helper_source':(NATIVE/'terrain-cell-head.rs').read_text()}],
        'modules':[{'mod_id':'pluto_native_creative',**{k:x[k] for k in ['module_id','file','name','fingerprint']},'source':(NATIVE/x['source']).read_text()} for x in m['modules']]}
    def test_exact_raw_and_exact_packaged_ids(self):
        p=self.plan();guard.check(p,guard.metadata())
        for key,idkey in [('injections','injection_id'),('modules','module_id')]:
            for entry in p[key]:entry[idkey]='pluto_native_creative-'+entry[idkey]
        guard.check(p,guard.metadata())
    def test_foreign_alias_interior_or_return_hooks_rejected(self):
        for kind in ['HEAD','RETURN','WRAP_OPERATION','MODIFY_VARIABLE','AFTER_CALL']:
            p=self.plan();h=copy.deepcopy(p['injections'][0]);h.update(target='alias::mesh_reusing',kind=kind,mod_id='other');p['injections'].append(h)
            with self.assertRaises(ValueError):guard.check(p,guard.metadata())
        p=self.plan();h=copy.deepcopy(p['injections'][0]);h.update(target='alias::face',node=[76,2],function_fingerprint='different');p['injections'].append(h)
        with self.assertRaises(ValueError):guard.check(p,guard.metadata())
    def test_contract_ownership_input_and_source_changes_rejected(self):
        changes=[lambda p:p.update(schema=5),lambda p:p.update(inputs={}),lambda p:p['injections'][0].update(priority=0),lambda p:p['injections'][0].update(cancellable=False),lambda p:p['injections'][0].update(bindings=[]),lambda p:p['injections'][0].update(helper_source='changed'),lambda p:p['injections'][0].update(injection_id='foreign-terrain-cell-head'),lambda p:p['modules'][0].update(mod_id='foreign'),lambda p:p['modules'][0].update(source='changed'),lambda p:p['modules'].append(copy.deepcopy(p['modules'][0]))]
        for change in changes:
            p=self.plan();change(p)
            with self.assertRaises(ValueError):guard.check(p,guard.metadata())
    def test_hooks_and_registry_have_one_owner(self):
        m=tomllib.loads((NATIVE/'horizon-mixin.toml').read_text())
        hooks=[h for h in m['injection'] if h['id'].startswith('terrain-cell')]
        self.assertEqual(len(hooks),1);self.assertEqual(hooks[0]['target'],guard.metadata()['target'])
        self.assertFalse(any(s['id'].startswith('terrain-cell') for s in m.get('state',[])))
        for name in ['terrain-cell-providers.rs','terrain-cell-host.rs','terrain-cell-head.rs']:
            source=(NATIVE/name).read_text();self.assertNotRegex(source,r'(?m)^\s*(?:#!\s*\[|//!|/\*!)')
            self.assertNotIn('pluto_api::fluid',source);self.assertNotIn('horizon_client::game',source)
        self.assertEqual((NATIVE/'terrain-cell-providers.rs').read_text().count('static REGISTRY:'),1)
    def test_host_has_no_raw_vertex_shader_or_per_cell_cache_lookup(self):
        source=(NATIVE/'terrain-cell-host.rs').read_text()
        self.assertEqual(source.count('with_owners('),1)
        self.assertNotIn('TerrainVertex::new',source);self.assertNotIn('fluid_face(',source)
        self.assertNotIn('Mutex',source);self.assertNotIn('RwLock',source)
        self.assertIn('String::new()',source);self.assertNotIn('.quad.clone()',source)
        self.assertIn('ControlFlow::Continue(())',source)
        self.assertIn('*out = mesh;',source)
    def test_actual_package_contains_exact_cell_sources(self):
        spec=importlib.util.spec_from_file_location('terrain_cell_package',ROOT/'tools/package_native.py');package=importlib.util.module_from_spec(spec);spec.loader.exec_module(package)
        _,files=package._bundle(json.loads((ROOT/'horizon-native.json').read_text()))
        for name in ['terrain-cell-providers.rs','terrain-cell-host.rs','terrain-cell-head.rs','terrain-binding-host.rs']:
            self.assertEqual(files['pluto_native_creative/'+name],(NATIVE/name).read_bytes())
if __name__=='__main__':unittest.main()
