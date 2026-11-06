"""Owned-field patch delegation must restore the complete old methods exactly."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
META = json.loads((ROOT / 'tconstruct/tests/rust/tank_compound_legacy.json').read_text())

def digest(value): return hashlib.sha256(value.encode()).hexdigest()
def span(source):
    start = source.index('    pub(super) fn to_nbt_preserving(')
    end = source.index('{', start) + 1
    depth = 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return start, end

def check_scope(case, path, source):
    entry = META['files'][path]
    oracle = (ROOT / entry['oracle_path']).read_text()
    case.assertEqual(digest(oracle), entry['oracle_sha256'])
    old = oracle.rstrip('\n')
    case.assertEqual(digest(old), entry['original_method_sha256'])
    a, b = span(source)
    case.assertEqual(digest(source[a:b]), entry['adopted_method_sha256'])
    case.assertEqual(digest(source[:a] + old + source[b:]), entry['original_source_sha256'])

class TankCompound(unittest.TestCase):
    def test_complete_frozen_methods_restore_whole_original_files(self):
        for path in META['files']:
            check_scope(self, path, (ROOT / path).read_text())

    def test_adoption_uses_existing_helper_and_no_new_schema(self):
        for path in META['files']:
            text = (ROOT / path).read_text(); a, b = span(text)
            self.assertIn('horizon_client::pluto_nbt_storage::patch_compound(', text[a:b])
            self.assertNotIn('to_string_lossy()', text[a:b])
            check_scope(self, path, text)
        helper = 'pluto-lib/mixins/pluto_native_creative/nbt-storage.rs'
        self.assertEqual(hashlib.sha256((ROOT / helper).read_bytes()).hexdigest(), META['unchanged_dependencies'][helper])

    def test_opaque_alias_and_raw_slot_policy_drift_are_rejected(self):
        changes = {
            'source-smeltery-tank.rs': [('row != &old_normal[index]', 'false'),
                ('"Capacity", "Fluids"', '"Capacity", "new_alias"'),
                ('old.tag == fluid.tag', 'true'), ('id.to_string().ok()?', 'id.to_string_lossy()')],
            'source-casting-tank.rs': [('slots[index] != old', 'false'),
                ('if unchanged { continue; }', 'if false { continue; }'),
                ('!self.lossless()', 'false'), ('"lastRedstone"', '"new_alias"')],
        }
        for name, edits in changes.items():
            path = 'tconstruct/mixins/tconstruct/' + name; source = (ROOT / path).read_text()
            for old, new in edits:
                with self.subTest(path=path, edit=old):
                    self.assertIn(old, source)
                    with self.assertRaises(AssertionError): check_scope(self, path, source.replace(old, new, 1))

    def test_only_method_declaration_is_renamed_in_real_type_fixture(self):
        runner = (ROOT / 'tconstruct/tests/run_tank_compound.py').read_text()
        for snippet in ('production +', 'actual.read_bytes()[:len(production)] != production',
                        "oracle.replace('fn to_nbt_preserving(', 'fn legacy_to_nbt_preserving(', 1)",
                        'add_inventory_api(manifest, root)', 'lock_fixture_to_host(manifest, host, root)',
                        'verify_fixture_lock(manifest, host, root)', 'run_locked_fixture(manifest',
                        'horizon-items', 'horizon-world', 'horizon-client-core'):
            self.assertIn(snippet, runner)
        self.assertNotIn('pub struct ItemStack', runner)

    def test_bounded_matrix_and_wire_tests_remain_required(self):
        tests = (ROOT / 'tconstruct/tests/rust/tank_compound.rs').read_text()
        runner = (ROOT / 'tconstruct/tests/run_tank_compound.py').read_text()
        for name in ('smeltery_complete_legacy_method_matches_owned_patch_matrix',
                     'casting_complete_legacy_method_matches_raw_slot_and_alias_matrix',
                     'unknown_utf16_and_float_bits_stay_on_exact_row_identity',
                     'casting_rejects_root_and_nested_retained_wire_without_changing_inputs',
                     'native_compound_has_last_value_and_sorted_exact_java_keys'):
            self.assertIn('fn ' + name + '()', tests); self.assertIn(name, runner)
        for count in (308, 216, 10): self.assertIn(f'assert_eq!(cases, {count});', tests)
        self.assertIn('assert_eq!(*raw, old_raw)', tests)
        self.assertIn('Wire::new(baseline', tests)

if __name__ == '__main__': unittest.main()
