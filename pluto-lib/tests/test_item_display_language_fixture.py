"""Small source/preparer guards only. No Cargo or native code executes here."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('item_display_prepare', ROOT / 'tools/prepare_item_display_language_fixture.py')
PREP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PREP)

class SourceGuards(unittest.TestCase):
    def test_fixture_is_frozen_eight_tests_with_actual_native_crates(self):
        source = (ROOT / 'tests/native/item_display_language.rs').read_text()
        record = json.loads((ROOT / 'docs/item-display-language-inputs-20261106.json').read_text())
        self.assertEqual(PREP.digest(source.encode()), record['fixture_sha256'])
        self.assertEqual(source.count('#[test]'), 8)
        self.assertIn('horizon_items::{item_nbt, items::ItemStack}', source)
        self.assertIn('pub use horizon_client_core::{lang, text}', source)
        self.assertEqual(record['expected_tests'], 22)
        names = [name for gate in record['result_groups'] for name in gate['names']]
        self.assertEqual([g['expected'] for g in record['result_groups']], [8, 6, 8])
        self.assertEqual(len(names), len(set(names)))
        self.assertEqual(len(names), 22)
        for name in names:
            self.assertEqual(sum(g['prefix'] in name for g in record['result_groups']), 1)
        for name in record['result_groups'][0]['names']:
            self.assertIn('fn ' + name.rsplit('::', 1)[1] + '(', source)
        self.assertFalse(record['rust_run'])
    def test_exact_fragment_boundaries_reject_duplicates(self):
        self.assertEqual(PREP.section('before BEGINbodyEND after', 'BEGIN', 'END'), 'BEGINbody')
        with self.assertRaises(ValueError):
            PREP.section('BEGIN BEGIN END', 'BEGIN', 'END')
    def test_exact_fragment_boundaries_reject_reverse_order(self):
        with self.assertRaises(ValueError):
            PREP.section('END BEGIN', 'BEGIN', 'END')
    def test_file_inventory_rejects_symlinks(self):
        with tempfile.TemporaryDirectory(prefix='pluto-item-source-guard-') as tmp:
            root = Path(tmp)
            (root/'actual').write_text('data')
            (root/'alias').symlink_to(root/'actual')
            with self.assertRaises(ValueError):
                PREP.files(root)
    def test_file_inventory_exposes_extra_config_files(self):
        with tempfile.TemporaryDirectory(prefix='pluto-item-source-guard-') as tmp:
            root = Path(tmp)
            (root/'lib.rs').write_text('')
            before = PREP.files(root)
            (root/'.cargo').mkdir()
            (root/'.cargo/config.toml').write_text('[build]\nrustflags=[]\n')
            after = PREP.files(root)
            self.assertNotEqual(before, after)
            self.assertIn('.cargo/config.toml', after)
    def test_proof_does_not_claim_game_or_default_component_propagation(self):
        record = json.loads((ROOT/'docs/item-display-language-inputs-20261106.json').read_text())
        self.assertEqual(record['official'], PREP.PIN)
        self.assertTrue(all(r['status']=='source_only_not_compiled_in_fixture' for r in record['source_only_routes']))
        self.assertFalse(record['metadata_run'])
        self.assertFalse(record['game_run'])
        source = (ROOT/'tests/native/item_display_language.rs').read_text()
        self.assertNotIn('lang::set(', source)
        self.assertNotIn('struct ItemStack', source)
        self.assertNotIn('struct Game', source)
        self.assertNotIn('pub struct Names', source)

if __name__ == '__main__':
    unittest.main()
