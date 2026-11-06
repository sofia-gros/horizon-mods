"""Boundaries of the explicitly Java-specific, pure recipe-order extraction."""
import hashlib
import json
import re
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / 'pluto-lib/tests/java-hashmap'
META = json.loads((FIXTURE / 'provenance.json').read_text())
OLD = (FIXTURE / 'legacy_recipe_order.rs').read_text()
CURRENT = ROOT / 'tconstruct/mixins/tconstruct/recipe-order.rs'


def policy_segments(source):
    """These are consumer decisions, not delegated Java map mechanics."""
    components = source[source.index('fn components('):source.index('fn hash(')]
    sort_start = source.index('    let mut ordered = ')
    sort_end = source.index('    ordered.dedup_by(')
    ordered = source[sort_start:source.index('\n', sort_end)]
    tests = source[source.index('#[cfg(test)]'):]
    return components, ordered, tests


def check_policy(case, source):
    case.assertEqual(policy_segments(source), policy_segments(OLD))
    case.assertIn('let value = string_hash(namespace).wrapping_mul(31).wrapping_add(string_hash(path));\n    Ok(value)', source)
    case.assertIn('map_err(|_| OrderError::TreeBin)?;', source)
    case.assertIn('ordered[index].2.clone()', source)


class JavaHashMapBoundaries(unittest.TestCase):
    def test_frozen_original_and_java_reference_are_exact(self):
        self.assertEqual(hashlib.sha256(OLD.encode()).hexdigest(), META['legacy_sha256'])
        java = ROOT / 'tconstruct/tests/reference/RecipeOrderOracle.java'
        self.assertEqual(hashlib.sha256(java.read_bytes()).hexdigest(), META['java_oracle_sha256'])

    def test_id_sort_dedup_and_existing_source_tests_stay_consumer_owned(self):
        check_policy(self, CURRENT.read_text())

    def test_owned_policy_guard_rejects_independent_drift(self):
        source = CURRENT.read_text()
        for original, changed in [('ordered.sort();', 'ordered.reverse();'),
                                  ("split_once(':')", "split_once('/')"),
                                  ('wrapping_mul(31)', 'wrapping_mul(33)'),
                                  ('ordered[index].2.clone()', 'ordered[0].2.clone()')]:
            self.assertIn(original, source)
            with self.assertRaises(AssertionError):
                check_policy(self, source.replace(original, changed, 1))

    def test_both_fixtures_link_the_single_actual_api_package(self):
        for manifest in [FIXTURE / 'Cargo.toml', ROOT / 'tconstruct/tests/recipe-order/Cargo.toml']:
            data = tomllib.loads(manifest.read_text())
            dep = data['dependencies']['pluto-inventory-api']['path']
            self.assertEqual((manifest.parent / dep).resolve(), ROOT / 'pluto-lib/mixins/pluto_native_creative/inventory-api')
            lock = tomllib.loads(manifest.with_name('Cargo.lock').read_text())
            api = [p for p in lock['package'] if p['name'] == 'pluto-inventory-api']
            self.assertEqual(len(api), 1)
            self.assertNotIn('source', api[0])
            self.assertEqual(api[0]['dependencies'], ['serde_json'])

    def test_compiled_source_paths_resolve_to_real_consumer_and_frozen_oracle(self):
        source = FIXTURE / 'src/lib.rs'
        paths = re.findall(r'#\[path = "([^"]+)"\]', source.read_text())
        actual = {(source.parent / path).resolve() for path in paths}
        self.assertEqual(actual, {CURRENT, FIXTURE / 'legacy_recipe_order.rs'})
        self.assertTrue(all(path.is_file() for path in actual))

    def test_generic_source_has_no_resource_schema_or_native_hook(self):
        src = ROOT / 'pluto-lib/mixins/pluto_native_creative/inventory-api/src'
        self.assertIn('pub mod java_hashmap;', (src / 'lib.rs').read_text())
        module = (src / 'java_hashmap.rs').read_text()
        for forbidden in ['tconstruct:', 'minecraft:', 'forge:', 'serde_json', 'thread_local!', 'Mutex', 'horizon_']:
            self.assertNotIn(forbidden, module)
        self.assertIn('Result<Vec<usize>, OrderError>', module)

    def test_existing_full_java_verifier_uses_real_locked_cargo(self):
        runner = (ROOT / 'tconstruct/tools/verify_recipe_order.py').read_text()
        self.assertNotIn("['rustc'", runner)
        self.assertIn("['cargo', 'run', '--quiet', '--locked', '--offline'", runner)
        for path in [FIXTURE / 'src/lib.rs', ROOT / 'tconstruct/tests/rust/recipe_order_dump.rs']:
            self.assertIn('pub use pluto_inventory_api as pluto_inventory;', path.read_text())


if __name__ == '__main__':
    unittest.main()
