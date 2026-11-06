"""Source/packaging boundaries for the pure ordered predicate fold extraction."""
import hashlib
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / 'pluto-lib/tests/predicate-fold'
ORACLE = json.loads((FIXTURE / 'legacy-oracle.json').read_text())


class PredicateFoldBoundaries(unittest.TestCase):
    def test_frozen_original_fragments_are_byte_exact(self):
        fixture = (FIXTURE / 'src/lib.rs').read_text()
        for key, text in ORACLE['frozen_fragments'].items():
            self.assertEqual(hashlib.sha256(text.encode()).hexdigest(), ORACLE['fragment_sha256'][key])
            self.assertIn(text, fixture)

    def test_only_the_five_original_fold_bodies_changed(self):
        groups = {'source-conditions.rs': ['conditions'],
                  'transaction.rs': ['transaction_or', 'transaction_and'],
                  'source-melting-recipes.rs': ['melting_any', 'melting_all']}
        for name, keys in groups.items():
            current = (ROOT / 'tconstruct/mixins/tconstruct' / name).read_text()
            for key in keys:
                new = ORACLE['consumer_replacements'][key]
                self.assertEqual(current.count(new), 1)
                current = current.replace(new, ORACLE['frozen_fragments'][key], 1)
            self.assertEqual(hashlib.sha256(current.encode()).hexdigest(), ORACLE['source_sha256'][name])

    def test_strict_station_unknown_semantics_are_untouched(self):
        source = (ROOT / 'tconstruct/mixins/tconstruct/source-station-recipes.rs').read_bytes()
        self.assertEqual(hashlib.sha256(source).hexdigest(), ORACLE['strict_station_sha256'])

    def test_real_shared_package_is_used_by_all_cargo_fixtures(self):
        for manifest in [FIXTURE / 'Cargo.toml', ROOT / 'tconstruct/tests/condition-snapshot/Cargo.toml',
                         ROOT / 'tconstruct/tests/soul-glass/Cargo.toml']:
            data = tomllib.loads(manifest.read_text())
            dependency = data['dependencies']['pluto-inventory-api']
            target = (manifest.parent / dependency['path']).resolve()
            self.assertEqual(target, ROOT / 'pluto-lib/mixins/pluto_native_creative/inventory-api')
            lock = tomllib.loads(manifest.with_name('Cargo.lock').read_text())
            found = [p for p in lock['package'] if p['name'] == 'pluto-inventory-api']
            self.assertEqual(len(found), 1)
            self.assertNotIn('source', found[0])
            self.assertEqual(found[0]['dependencies'], ['serde_json'])

    def test_source_module_is_generic_and_has_no_native_owner(self):
        base = ROOT / 'pluto-lib/mixins/pluto_native_creative/inventory-api/src'
        self.assertIn('pub mod predicate_fold;', (base / 'lib.rs').read_text())
        source = (base / 'predicate_fold.rs').read_text()
        for forbidden in ['tconstruct:', 'forge:', 'serde_json', 'thread_local!', 'Mutex', 'static ']:
            self.assertNotIn(forbidden, source)


if __name__ == '__main__':
    unittest.main()
