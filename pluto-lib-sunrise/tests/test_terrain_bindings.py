"""Preparation/packaging guard checks, not native renderer execution."""
import copy
import importlib.util
import json
from pathlib import Path
import tomllib
import unittest
ROOT=Path(__file__).resolve().parents[1];NATIVE=ROOT/'mixins/pluto_native_creative'
spec=importlib.util.spec_from_file_location('terrain_guard',ROOT/'tools/check_terrain_binding_plan.py');guard=importlib.util.module_from_spec(spec);spec.loader.exec_module(guard)
class TerrainBindingChecks(unittest.TestCase):
    def plan(self):
        m=guard.metadata()
        return {'inputs':m['source_sha256'],'injections':[{'mod_id':'pluto_native_creative','injection_id':m['injection_id'],'target':m['target'],**m['location'],'function_fingerprint':m['function_fingerprint'],'kind':'HEAD','cancellable':False,'bindings':m['bindings'],'helper_source':(NATIVE/m['helper']).read_text()}],
            'modules':[{'mod_id':'pluto_native_creative',**{k:x[k] for k in ['module_id','file','name','fingerprint']},'source':(NATIVE/x['source']).read_text()} for x in m['modules']]}
    def test_exact_plan(self):guard.check(self.plan(),guard.metadata())
    def test_exact_packaged_prefixes_pass(self):
        plan = self.plan()
        for hook in plan['injections']:
            hook['injection_id'] = 'pluto_native_creative-' + hook['injection_id']
        for module in plan['modules']:
            module['module_id'] = 'pluto_native_creative-' + module['module_id']
        guard.check(plan, guard.metadata())

    def test_foreign_ids_and_module_owners_fail_closed(self):
        for change in [
            lambda p: p['injections'][0].update(injection_id='foreign-fluid-render-before-assemble'),
            lambda p: p['injections'][0].update(mod_id='foreign'),
            lambda p: p['modules'][0].update(module_id='foreign-terrain-binding-host'),
            lambda p: p['modules'][0].update(mod_id='foreign'),
        ]:
            plan = self.plan()
            change(plan)
            with self.assertRaises(ValueError):
                guard.check(plan, guard.metadata())

    def test_duplicate_component_and_packaged_module_are_not_two_owners(self):
        plan = self.plan()
        duplicate = copy.deepcopy(plan['modules'][0])
        duplicate['module_id'] = 'pluto_native_creative-' + duplicate['module_id']
        plan['modules'].append(duplicate)
        with self.assertRaises(ValueError):
            guard.check(plan, guard.metadata())

    def test_foreign_alias_hooks_fail_closed(self):
        for kind in ['HEAD','RETURN','MODIFY_VARIABLE','AFTER_CALL','WRAP_OPERATION']:
            p=self.plan();h=copy.deepcopy(p['injections'][0]);h.update(mod_id='other',target='alias::assemble',kind=kind);p['injections'].append(h)
            with self.assertRaises(ValueError):guard.check(p,guard.metadata())
    def test_missing_changed_owner_helper_or_native_source_fails_closed(self):
        for change in [lambda p:p.update(injections=[]),lambda p:p['injections'][0].update(cancellable=True),lambda p:p['injections'][0].update(bindings=[]),lambda p:p['injections'][0].update(helper_source='changed'),lambda p:p.update(inputs={}),lambda p:p.update(modules=[]),lambda p:p['modules'][0].update(source='changed')]:
            p=self.plan();change(p)
            with self.assertRaises(ValueError):guard.check(p,guard.metadata())
    def test_no_cell_draw_or_reverse_dependency_hook(self):
        m=tomllib.loads((NATIVE/'sunrise-mod.toml').read_text())
        names={'terrain-bindings','terrain-binding-host','fluid-render-prepare'}
        self.assertEqual({x['id'] for x in m['module'] if x['id'] in names},names)
        hooks=[h for h in m['injection'] if h['id'].startswith(('terrain-binding','fluid-render'))]
        self.assertEqual(len(hooks),1);self.assertEqual(hooks[0]['target'],guard.metadata()['target']);self.assertEqual(hooks[0]['kind'],'HEAD')
        self.assertFalse(any(s['id'].startswith(('terrain-binding','fluid-render')) for s in m.get('state',[])))
        for name in ['terrain-bindings.rs','terrain-binding-host.rs']:
            source=(NATIVE/name).read_text();self.assertNotIn('pluto_api::fluid',source);self.assertNotIn('sunrise_client::game',source)
        source=(NATIVE/'fluid-render-prepare.rs').read_text();self.assertIn('registry_snapshot()',source);self.assertIn('revision.is_current()',source)
    def test_actual_package_contains_exact_modules_and_revised_single_registry(self):
        spec=importlib.util.spec_from_file_location('terrain_package',ROOT/'tools/package_native.py');package=importlib.util.module_from_spec(spec);spec.loader.exec_module(package)
        _,files=package._bundle(json.loads((ROOT/'horizon-native.json').read_text()))
        for name in ['terrain-bindings.rs','terrain-binding-host.rs','fluid-render-prepare.rs','fluid-render-before-assemble.rs','api/src/fluid.rs']:
            self.assertEqual(files['pluto_native_creative/'+name],(NATIVE/name).read_bytes())
        source=(NATIVE/'api/src/fluid.rs').read_text();self.assertEqual(source.count('static REGISTRY:'),1)
class NativeTerrainAssemblyTests(unittest.TestCase):
    def test_native_owners_sources_and_no_catalog_attachment(self):
        import hashlib
        import tempfile
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / guard.NATIVE_FILE
            source.parent.mkdir(parents=True)
            source.write_bytes(b'native assembly')
            expected = hashlib.sha256(source.read_bytes()).hexdigest()
            with patch.object(guard, 'NATIVE_SHA256', expected):
                plan = dict(schema=6, inputs={guard.NATIVE_FILE: expected}, injections=[],
                            modules=[{'mod_id': 'pluto_lib', 'file': 'unrelated.rs'}], states=[])
                meta = guard.metadata(plan)
                guard.check(plan, meta)
                guard.check_source(root, meta)
                candidates = [{'file': guard.NATIVE_FILE}, {'target': 'alias::assemble'},
                              {'target': 'alias::PackResources::load'}, {'mod_id': 'pluto_native_creative'}]
                candidates += [{'name': name} for name in (
                    'pluto_terrain_binding_host', 'pluto_terrain_bindings', 'pluto_fluid_render_prepare')]
                for section in ('injections', 'modules', 'states'):
                    for entry in candidates:
                        plan[section].append(entry)
                        with self.assertRaisesRegex(ValueError, 'unattached'):
                            guard.check(plan, meta)
                        plan[section].pop()
                plan['inputs'].clear()
                with self.assertRaisesRegex(ValueError, 'plan source differs'):
                    guard.check(plan, meta)
                plan['inputs'][guard.NATIVE_FILE] = expected
                with self.assertRaisesRegex(ValueError, 'metadata'):
                    guard.check(plan, guard.metadata())
                plan['schema'] = 7
                with self.assertRaisesRegex(ValueError, 'schema'):
                    guard.check(plan, meta)
                source.write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'source drifted'):
                    guard.check_source(root, meta)


if __name__=='__main__':unittest.main()
