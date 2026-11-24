import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import tomllib
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / 'mixins/pluto_native_creative'
BUNDLE = ROOT / 'mod'


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'tools' / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


GUARD = load('check_crafting_match_plan')
SHARED = load('check_recipe_placement_plan')
PACKAGE = load('package_native')
BOUNDARIES = json.loads((ROOT / 'tools/crafting-match-boundaries.json').read_text(encoding='utf-8'))
SPECIAL = [b for b in BOUNDARIES['boundaries'] if b['injection_id'] == 'crafting-special']
BUNDLE_SPECIAL = [b for b in BOUNDARIES['bundle_attachments'] if b.get('injection_id') == 'pluto-crafting-special']
FIND_WITH = 'horizon_client_lib::crafting::Recipes::find_with'


class CraftingSpecialBoundaries(unittest.TestCase):
    def test_registry_is_generic_and_result_only(self):
        source = (NATIVE / 'inventory-api/src/crafting/special.rs').read_text(encoding='utf-8')
        for forbidden in ('horizon_', 'tconstruct:', 'RecipeIndex', 'static mut'):
            self.assertNotIn(forbidden, source)
        self.assertEqual(source.count('static PROVIDERS:'), 1)
        self.assertIn('advancements::predicates::ComponentValue', source)
        self.assertIn('provider that answers the grid wins', source)

    def test_adapter_fills_only_native_misses(self):
        source = (NATIVE / 'crafting-match.rs').read_text(encoding='utf-8')
        self.assertIn('pub fn evaluate_special', source)
        self.assertLess(source.index('special::has_providers()'), source.index('assemble_if_matches'))
        self.assertLess(source.index('assemble_if_matches'), source.index('special::evaluate'))
        for forbidden in ('tconstruct:', '.send(', 'menu_session('):
            self.assertNotIn(forbidden, source)
        source = (NATIVE / 'crafting-special.rs').read_text(encoding='utf-8')
        self.assertIn('fn pluto_crafting_special(', source)
        self.assertIn('ControlFlow::Break(result)', source)
        self.assertIn('ControlFlow::Continue(())', source)
        self.assertIn('evaluate_special(recipes, grid, size, map_scale)', source)

    def test_bundle_resolves_markers_instead_of_a_find_with_head(self):
        # The bundle composition cannot declare the find_with HEAD (its
        # borrowed result is unspellable), so providers answer through
        # marker entries resolved by the crafting-match veto instead.
        adapter = (BUNDLE / 'pluto-crafting-match.rs').read_text(encoding='utf-8')
        self.assertNotIn('pub fn evaluate_special', adapter)
        self.assertIn('fn evaluate_marker', adapter)
        self.assertIn('special::provider_ids()', adapter)
        self.assertIn('special::evaluate_provider', adapter)
        self.assertIn('Verdict::Unmatched', adapter)
        self.assertLess(adapter.index('fn evaluate_marker'), adapter.index('special::evaluate_provider'))
        for forbidden in ('tconstruct:', '.send(', 'menu_session('):
            self.assertNotIn(forbidden, adapter)
        veto = (BUNDLE / 'pluto-crafting-match-veto-head.rs').read_text(encoding='utf-8')
        self.assertIn('Verdict::Unmatched', veto)
        self.assertIn('ControlFlow::Break(None)', veto[veto.index('Verdict::Unmatched'):])
        files = (BUNDLE / 'pluto-crafting-special-files-head.rs').read_text(encoding='utf-8')
        self.assertIn('fn pluto_crafting_special_files(', files)
        self.assertIn('crafting_special_repairitem', files)
        self.assertIn('provider_ids()', files)
        synced = (BUNDLE / 'pluto-crafting-special-synced-head.rs').read_text(encoding='utf-8')
        self.assertIn('fn pluto_crafting_special_synced(', synced)
        self.assertIn('recipe_tables.synced', synced)

    def test_manifests_declare_a_cancellable_head_without_require(self):
        standalone = tomllib.loads((NATIVE / 'horizon-mixin.toml').read_text(encoding='utf-8'))
        hooks = [h for h in standalone['injection'] if h['id'] == 'crafting-special']
        self.assertEqual(len(hooks), 1)
        self.assertEqual(hooks[0]['target'], FIND_WITH)
        self.assertEqual(hooks[0]['kind'], 'HEAD')
        self.assertTrue(hooks[0]['cancellable'])
        self.assertEqual(hooks[0]['helper'], 'crafting-special.rs')
        self.assertNotIn('require', hooks[0])
        self.assertNotIn('selector', hooks[0])
        bundle = tomllib.loads((BUNDLE / 'horizon-mixin.toml').read_text(encoding='utf-8'))
        hooks = [h for h in bundle['injection'] if h['id'] == 'pluto-crafting-special']
        self.assertEqual(hooks, [])
        files = [h for h in bundle['injection'] if h['id'] == 'pluto-crafting-special-files']
        self.assertEqual(len(files), 1)
        self.assertEqual(files[0]['target'], "horizon_client::game::native_hooks::Game<'_>::native_data_pack_files")
        self.assertEqual(files[0]['kind'], 'HEAD')
        self.assertEqual(files[0]['helper'], 'pluto-crafting-special-files-head.rs')
        self.assertNotIn('require', files[0])
        self.assertNotIn('selector', files[0])
        synced = [h for h in bundle['injection'] if h['id'] == 'pluto-crafting-special-synced']
        self.assertEqual(len(synced), 1)
        self.assertEqual(synced[0]['target'], "horizon_client::game::native_recipes::Game<'_>::recipes_received_now")
        self.assertEqual(synced[0]['kind'], 'HEAD')
        self.assertEqual(synced[0]['helper'], 'pluto-crafting-special-synced-head.rs')
        self.assertNotIn('require', synced[0])
        self.assertNotIn('selector', synced[0])

    def test_boundary_entries_pin_the_actual_sources(self):
        self.assertEqual(len(SPECIAL), 1)
        self.assertEqual(len(BUNDLE_SPECIAL), 1)
        for meta, helpers in ((SPECIAL[0], NATIVE), (BUNDLE_SPECIAL[0], BUNDLE)):
            self.assertEqual(meta['target'], FIND_WITH)
            self.assertEqual(meta['kind'], 'HEAD')
            self.assertTrue(meta['cancellable'])
            self.assertEqual(meta['file_sha256'], '298e65ea0417a37ccc73c29c23e25b396c369faeb9b17f0fef7f32b097c2243e')
            self.assertEqual(GUARD.digest((helpers / meta['helper']).read_bytes()), meta['helper_sha256'])
        self.assertEqual(SPECIAL[0]['priority'], 100)
        self.assertEqual(BUNDLE_SPECIAL[0]['priority'], 0)
        # The bundle guard requires every reviewed attachment exactly once
        # once the inventory gates are composed; the special fill is one.
        keys = {m.get('injection_id', m.get('module_id')) for m in SHARED.bundle_entries()}
        self.assertIn('pluto-crafting-special', keys)

    def test_actual_archive_contains_exact_source(self):
        with tempfile.TemporaryDirectory() as d:
            output = Path(d) / 'special.hnmod'
            PACKAGE.build(output)
            with zipfile.ZipFile(output) as archive:
                for file in ('inventory-api/src/crafting.rs', 'inventory-api/src/crafting/special.rs', 'crafting-special.rs'):
                    self.assertEqual(archive.read('mixins/pluto_native_creative/pluto_native_creative/' + file), (NATIVE / file).read_bytes())

    def metadata(self):
        return [dict(target=FIND_WITH, injection_id='crafting-special', file='native.rs', kind='HEAD', cancellable=True, priority=100, bindings=[], function_fingerprint='fingerprint', helper='helper.rs')]

    def plan(self, metadata):
        return dict(schema=6, injections=[dict(m, mod_id='pluto_native_creative') for m in metadata])

    def test_all_competing_phases_fail_closed(self):
        metadata = self.metadata()
        GUARD.check_order(self.plan(metadata), metadata)
        for kind in ('HEAD', 'RETURN', 'OVERWRITE', 'BEFORE_CALL', 'AFTER_CALL', 'MODIFY_ARG', 'WRAP_OPERATION'):
            plan = self.plan(metadata)
            plan['injections'].append(dict(target=FIND_WITH, kind=kind, mod_id='other', injection_id='other'))
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                GUARD.check_order(plan, metadata)

    def test_absent_spoofed_or_drifted_boundary_rejected(self):
        metadata = self.metadata()
        for key, value in (('mod_id', 'other'), ('injection_id', 'prefix-crafting-special'), ('cancellable', False), ('kind', 'RETURN'), ('function_fingerprint', 'changed'), ('priority', -1), ('bindings', [{}]), ('file', 'other.rs')):
            plan = self.plan(metadata)
            plan['injections'][0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                GUARD.check_order(plan, metadata)
        for plan in (dict(schema=6, injections=[]), dict(schema=7, injections=self.plan(metadata)['injections'])):
            with self.assertRaises(ValueError):
                GUARD.check_order(plan, metadata)


if __name__ == '__main__':
    unittest.main()
