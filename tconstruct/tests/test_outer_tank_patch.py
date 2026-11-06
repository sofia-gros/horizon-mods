"""Three exact outer-tank edits; every other caller byte remains unchanged."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
META = json.loads((ROOT / 'tconstruct/tests/rust/outer_tank_legacy.json').read_text())

def digest(text): return hashlib.sha256(text.encode()).hexdigest()

def span(source, declaration):
    start = source.index(declaration)
    end = source.index('{', start) + 1
    depth = 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return start, end

def check_file(case, path, source):
    edits = []
    for entry in META['methods'].values():
        if entry['path'] != path: continue
        oracle = (ROOT / entry['oracle_path']).read_text()
        case.assertEqual(digest(oracle), entry['oracle_sha256'])
        old = oracle.rstrip('\n')
        case.assertEqual(digest(old), entry['original_method_sha256'])
        a, b = span(source, entry['declaration'])
        case.assertEqual(digest(source[a:b]), entry['adopted_method_sha256'])
        edits.append((a, b, old))
    for a, b, old in sorted(edits, reverse=True): source = source[:a] + old + source[b:]
    case.assertEqual(digest(source), META['original_files'][path])

class OuterTankPatch(unittest.TestCase):
    def test_complete_oracles_restore_both_entire_source_files(self):
        for path in META['original_files']: check_file(self, path, (ROOT / path).read_text())

    def test_dependencies_and_existing_scaffold_are_unchanged(self):
        for path, expected in META['unchanged_dependencies'].items():
            self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), expected)

    def test_controller_keeps_shape_guard_and_lazy_encoding_after_clone(self):
        entry = META['methods']['controller']; source = (ROOT / entry['path']).read_text()
        a, b = span(source, entry['declaration']); body = source[a:b]
        self.assertLess(body.index('Tag::Compound(_) = raw'), body.index('patch_compound(raw,'))
        self.assertIn('std::iter::once_with(|| ("tank", Some(state.tank.to_nbt())))', body)
        helper = (ROOT / 'pluto-lib/mixins/pluto_native_creative/nbt-storage.rs').read_text()
        self.assertLess(helper.index('let mut fields = fields.clone();'), helper.index('for (key, value) in updates'))

    def test_alias_lossless_and_save_publish_drift_are_rejected(self):
        mutations = {
            'cannon': [('if !self.lossless()', 'if false'), ('["Tank", "Fluid"]', '["Tank"]')],
            'drain': [('if !store::write(', 'if store::write('), ('if game.creative {', 'if !game.creative {')],
            'controller': [('std::iter::once_with(||', 'std::iter::once(')],
        }
        for key, replacements in mutations.items():
            path = META['methods'][key]['path']; source = (ROOT / path).read_text()
            for old, new in replacements:
                self.assertIn(old, source)
                with self.subTest(key=key, old=old), self.assertRaises(AssertionError):
                    check_file(self, path, source.replace(old, new, 1))

    def test_fixture_uses_complete_bodies_actual_modules_and_strict_lock(self):
        runner = (ROOT / 'tconstruct/tests/run_outer_tank_patch.py').read_text()
        for needle in ('production +', 'actual.read_bytes()[:len(production)] != production',
                       "oracle.replace('fn tank_to_nbt_preserving(', 'fn legacy_tank_to_nbt_preserving(', 1)",
                       'add_inventory_api(manifest, root)', 'lock_fixture_to_host(manifest, host, root)',
                       'verify_fixture_lock(manifest, host, root)', "['cargo', 'test', '--lib', '--locked', '--offline'",
                       "scaffold[:scaffold.index('#[test]')]"):
            self.assertIn(needle, runner)
        tests = (ROOT / 'tconstruct/tests/rust/outer_tank_patch.rs').read_text()
        for name in ('cannon_complete_old_method_matches_raw_and_state_matrix',
                     'cannon_retained_wire_and_opaque_item_contracts_match',
                     'controller_patch_matches_unknown_and_malformed_values',
                     'drain_complete_route_matches_saved_and_published_results'):
            self.assertIn('fn ' + name + '()', tests)
            self.assertIn(name, runner)

if __name__ == '__main__': unittest.main()
