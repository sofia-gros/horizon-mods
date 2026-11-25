import copy
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import tomllib
import unittest
import zipfile
ROOT=Path(__file__).resolve().parents[1]
NATIVE=ROOT/'mixins/pluto_native_creative'
def load(name):
    spec=importlib.util.spec_from_file_location(name,ROOT/'tools'/f'{name}.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
GUARD=load('check_recipe_placement_plan');PACKAGE=load('package_native')
class PlacementBoundaries(unittest.TestCase):
    def test_one_shared_registry_is_veto_only_and_host_independent(self):
        source=(NATIVE/'inventory-api/src/recipe_book/placement.rs').read_text()
        for no in ('horizon_', 'tconstruct:', 'Allow(', 'static mut', 'RecipeIndex'):self.assertNotIn(no,source)
        self.assertEqual(source.count('static POLICIES:'),1)
        self.assertIn('Silent',source);self.assertIn('Denied',source);self.assertIn('None means unavailable',source)
    def test_native_heads_do_not_duplicate_policy_or_grant_placement(self):
        manifest=tomllib.loads((NATIVE/'sunrise-mod.toml').read_text())
        hooks=[h for h in manifest['injection'] if h['id'].startswith('recipe-placement-')]
        self.assertEqual({h['id'] for h in hooks},{'recipe-placement-definition','recipe-placement-click'})
        for hook in hooks:self.assertEqual(hook['kind'],'HEAD');self.assertTrue(hook['cancellable'])
        source=(NATIVE/'recipe-placement-attempt.rs').read_text()
        for no in ('menu_session(', '.send(', 'RecipeIndex', 'tconstruct:'):self.assertNotIn(no,source)
        self.assertIn('ControlFlow::Break(())',source);self.assertIn('let _ = capability.show',source)
        compact=''.join(source.split());self.assertLess(compact.index('has_providers()'),compact.index('game.recipe_ui'))
    def test_actual_package_carries_exact_api_and_four_native_sources(self):
        with tempfile.TemporaryDirectory() as d:
            output=Path(d)/'placement.hnmod';PACKAGE.build(output)
            with zipfile.ZipFile(output) as z:
                for file in ['inventory-api/src/recipe_book/placement.rs','recipe-placement.rs','recipe-placement-definition.rs','recipe-placement-attempt.rs','recipe-placement-click.rs']:
                    self.assertEqual(z.read('mixins/pluto_native_creative/pluto_native_creative/'+file),(NATIVE/file).read_bytes())
    def meta(self):
        return [dict(target=t,injection_id=i,file='native.rs',kind='HEAD',cancellable=True,priority=100,bindings=[],function_fingerprint='fingerprint',helper='helper.rs') for t,i in [('lib::Recipes::placement','recipe-placement-definition'),('game::place_recipe','recipe-placement-click')]]
    def plan(self,meta):
        return dict(schema=6,injections=[dict(m,mod_id='pluto_native_creative') for m in meta])
    def test_all_competing_native_hooks_fail_closed_on_both_targets(self):
        meta=self.meta();GUARD.check_order(self.plan(meta),meta)
        for target in [m['target'] for m in meta]:
            for kind in ['HEAD','RETURN','OVERWRITE','BEFORE_CALL','AFTER_CALL','MODIFY_ARG','WRAP_OPERATION']:
                plan=self.plan(meta);plan['injections'].append(dict(target=target,kind=kind,mod_id='other',injection_id='other'))
                with self.subTest(target=target,kind=kind),self.assertRaises(ValueError):GUARD.check_order(plan,meta)
    def test_missing_spoofed_and_drifted_targets_fail_closed(self):
        meta=self.meta()
        for key,value in [('mod_id','other'),('injection_id','suffix-recipe-placement-definition'),('cancellable',False),('function_fingerprint','other'),('priority',-1),('bindings',[{}]),('file','other.rs')]:
            plan=self.plan(meta);plan['injections'][0][key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):GUARD.check_order(plan,meta)
        plan=self.plan(meta);plan['injections'].pop()
        with self.assertRaises(ValueError):GUARD.check_order(plan,meta)
    def test_native_and_helper_bytes_are_bound_to_plan(self):
        meta=self.meta();plan=self.plan(meta)
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);(root/'native.rs').write_text('official');(root/'helper.rs').write_text('helper')
            for m,h in zip(meta,plan['injections']):m.update(file_sha256=GUARD.digest(b'official'),helper_sha256=GUARD.digest(b'helper'));h['helper_source']='helper'
            plan['inputs']={'native.rs':GUARD.digest(b'official')};GUARD.verify_source(plan,root,meta,root)
            broken=copy.deepcopy(plan);broken['injections'][0]['helper_source']='other'
            with self.assertRaises(ValueError):GUARD.verify_source(broken,root,meta,root)
            (root/'native.rs').write_text('changed')
            with self.assertRaises(ValueError):GUARD.verify_source(plan,root,meta,root)
class NativePlacementTests(unittest.TestCase):
    def test_matching_and_station_dispatch_use_their_own_source_sets(self):
        from unittest.mock import patch
        for name, method in (('check_crafting_match_plan', 'assemble_if_matches'),
                             ('check_station_ingredient_plan', 'ingredient')):
            module = load(name)
            with self.subTest(guard=name), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                sources = {}
                for file in module.NATIVE_SOURCES:
                    source = root / file
                    source.parent.mkdir(parents=True, exist_ok=True)
                    source.write_bytes(b'native recipe fixture')
                    sources[file] = module.digest(source.read_bytes())
                with patch.object(module, 'NATIVE_SOURCES', sources):
                    plan = dict(schema=6, inputs=dict(sources), injections=[], states=[],
                                modules=[{'mod_id': 'pluto_lib', 'file': 'unrelated.rs'}])
                    module.check(plan, root)
                    plan['injections'].append({'target': 'alias::' + method})
                    with self.assertRaisesRegex(ValueError, 'unattached'):
                        module.check(plan, root)
                    plan['injections'].clear()
                    for file in sources:
                        expected = plan['inputs'].pop(file)
                        with self.assertRaisesRegex(ValueError, 'plan source differs'):
                            module.check(plan, root)
                        plan['inputs'][file] = expected

    def test_preserved_sources_and_all_attachment_kinds(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sources = {}
            for name in GUARD.NATIVE_SOURCES:
                source = root / name
                source.parent.mkdir(parents=True, exist_ok=True)
                source.write_bytes(b'native placement')
                sources[name] = GUARD.digest(source.read_bytes())
            with patch.object(GUARD, 'NATIVE_SOURCES', sources):
                plan = dict(schema=6, inputs=dict(sources), injections=[], states=[],
                            modules=[{'mod_id': 'pluto_lib', 'file': 'unrelated.rs'}])
                GUARD.check(plan, root)
                for name in sources:
                    source = root / name
                    source.write_bytes(b'changed')
                    with self.assertRaisesRegex(ValueError, 'workspace source differs'):
                        GUARD.check(plan, root)
                    source.write_bytes(b'native placement')
                    expected = plan['inputs'].pop(name)
                    with self.assertRaisesRegex(ValueError, 'plan source differs'):
                        GUARD.check(plan, root)
                    plan['inputs'][name] = expected
                for section in ('injections', 'modules', 'states'):
                    for entry in ([{'file': name} for name in sources]
                                  + [{'target': 'alias::Recipes::placement'},
                                     {'target': 'alias::Game::place_recipe'},
                                     {'mod_id': 'pluto_native_creative'}]):
                        plan[section].append(entry)
                        with self.assertRaisesRegex(ValueError, 'unattached'):
                            GUARD.check(plan, root)
                        plan[section].pop()
                plan['schema'] = 7
                with self.assertRaisesRegex(ValueError, 'schema'):
                    GUARD.check(plan, root)


if __name__=='__main__':unittest.main()
