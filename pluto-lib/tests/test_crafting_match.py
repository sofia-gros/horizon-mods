import copy
import importlib.util
from pathlib import Path
import tempfile
import tomllib
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / 'mixins/pluto_native_creative'


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'tools' / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


GUARD = load('check_crafting_match_plan')
PACKAGE = load('package_native')


class CraftingMatchBoundaries(unittest.TestCase):
    def test_generic_read_only_registry_reuses_facts(self):
        source = (NATIVE / 'inventory-api/src/crafting/matching.rs').read_text()
        for forbidden in ('horizon_', 'tconstruct:', 'RecipeIndex', 'Allow(', 'static mut'):
            self.assertNotIn(forbidden, source)
        self.assertEqual(source.count('static POLICIES:'), 1)
        self.assertIn('recipe_book::placement::{Form, Ingredient}', source)
        self.assertIn('advancements::predicates::ItemSnapshot', source)
        self.assertIn('None is unavailable', source)
        self.assertIn('No output replacement', source)

    def test_interest_precedes_component_capture_and_head_only_denies(self):
        source = (NATIVE / 'crafting-match.rs').read_text()
        self.assertLess(source.index('has_providers()'), source.index('match recipe'))
        self.assertLess(source.index('matching::prepare'), source.index('ItemFacts::capture'))
        self.assertLess(source.index('matching::prepare'), source.index('template.create()'))
        for forbidden in ('tconstruct:', 'RecipeIndex', '.send(', 'menu_session(', '&mut'):
            self.assertNotIn(forbidden, source)
        helper = (NATIVE / 'crafting-match-veto.rs').read_text()
        self.assertIn('ControlFlow::Break(None)', helper)
        self.assertIn('ControlFlow::Continue(())', helper)
        manifest = tomllib.loads((NATIVE / 'horizon-mixin.toml').read_text())
        hooks = [h for h in manifest['injection'] if h['id'] == 'crafting-match-veto']
        self.assertEqual(len(hooks), 1)
        self.assertEqual(hooks[0]['target'], 'horizon_client_lib::crafting::Recipes::assemble_if_matches')
        self.assertEqual(hooks[0]['kind'], 'HEAD')
        self.assertTrue(hooks[0]['cancellable'])

    def test_actual_archive_contains_exact_source(self):
        with tempfile.TemporaryDirectory() as d:
            output = Path(d) / 'matching.hnmod'
            PACKAGE.build(output)
            with zipfile.ZipFile(output) as archive:
                for file in ('inventory-api/src/crafting.rs', 'inventory-api/src/crafting/matching.rs', 'crafting-match.rs', 'crafting-match-veto.rs'):
                    self.assertEqual(archive.read('mixins/pluto_native_creative/pluto_native_creative/' + file), (NATIVE / file).read_bytes())

    def metadata(self):
        return [dict(target='lib::Recipes::assemble_if_matches', injection_id='crafting-match-veto', file='native.rs', kind='HEAD', cancellable=True, priority=100, bindings=[], function_fingerprint='fingerprint', helper='helper.rs')]

    def plan(self, metadata):
        return dict(schema=6, injections=[dict(m, mod_id='pluto_native_creative') for m in metadata])

    def test_all_competing_phases_fail_closed(self):
        metadata = self.metadata()
        GUARD.check_order(self.plan(metadata), metadata)
        for kind in ('HEAD', 'RETURN', 'OVERWRITE', 'BEFORE_CALL', 'AFTER_CALL', 'MODIFY_ARG', 'WRAP_OPERATION'):
            plan = self.plan(metadata)
            plan['injections'].append(dict(target=metadata[0]['target'], kind=kind, mod_id='other', injection_id='other'))
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                GUARD.check_order(plan, metadata)

    def test_absent_spoofed_or_drifted_boundary_rejected(self):
        metadata = self.metadata()
        for key, value in (('mod_id', 'other'), ('injection_id', 'prefix-crafting-match-veto'), ('cancellable', False), ('kind', 'RETURN'), ('function_fingerprint', 'changed'), ('priority', -1), ('bindings', [{}]), ('file', 'other.rs')):
            plan = self.plan(metadata)
            plan['injections'][0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                GUARD.check_order(plan, metadata)
        for plan in (dict(schema=6, injections=[]), dict(schema=7, injections=self.plan(metadata)['injections'])):
            with self.assertRaises(ValueError):
                GUARD.check_order(plan, metadata)

    def test_native_and_helper_identity_must_match_complete_plan(self):
        metadata = self.metadata()
        plan = self.plan(metadata)
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / 'native.rs').write_text('official')
            (root / 'helper.rs').write_text('helper')
            metadata[0].update(file_sha256=GUARD.digest(b'official'), helper_sha256=GUARD.digest(b'helper'))
            plan['injections'][0]['helper_source'] = 'helper'
            plan['inputs'] = {'native.rs': GUARD.digest(b'official')}
            GUARD.verify_source(plan, root, metadata, root)
            for field in ('helper_source', 'inputs'):
                broken = copy.deepcopy(plan)
                if field == 'helper_source':
                    broken['injections'][0][field] = 'changed'
                else:
                    broken[field] = {}
                with self.subTest(field=field), self.assertRaises(ValueError):
                    GUARD.verify_source(broken, root, metadata, root)
            (root / 'helper.rs').write_text('changed')
            with self.assertRaises(ValueError):
                GUARD.verify_source(plan, root, metadata, root)
            (root / 'helper.rs').write_text('helper')
            (root / 'native.rs').write_text('changed')
            with self.assertRaises(ValueError):
                GUARD.verify_source(plan, root, metadata, root)


if __name__ == '__main__':
    unittest.main()
