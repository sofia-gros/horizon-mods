"""Keep standalone API tests reproducible without assuming a warm CI cache."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]

class InventoryTestLockTests(unittest.TestCase):
    def test_lock_has_one_real_api_package_and_only_pinned_registry_dependencies(self):
        data = tomllib.loads((ROOT / 'tests/inventory-api.Cargo.lock').read_text())
        packages = data['package']
        local = [row for row in packages if 'source' not in row]
        self.assertEqual([(row['name'], row['version']) for row in local], [('pluto-inventory-api', '0.1.0')])
        self.assertEqual(local[0]['dependencies'], ['serde_json'])
        self.assertEqual(len({(row['name'], row['version']) for row in packages}), len(packages))
        for row in packages:
            if 'source' in row:
                self.assertEqual(row['source'], 'registry+https://github.com/rust-lang/crates.io-index')
                self.assertRegex(row['checksum'], r'^[0-9a-f]{64}$')

    def test_fluid_api_reuses_exact_registry_identities_and_real_manifest(self):
        inventory = tomllib.loads((ROOT / 'tests/inventory-api.Cargo.lock').read_text())
        fluid = tomllib.loads((ROOT / 'tests/fluid-api.Cargo.lock').read_text())
        self.assertEqual([row for row in fluid['package'] if 'source' in row],
                         [row for row in inventory['package'] if 'source' in row])
        local = [row for row in fluid['package'] if 'source' not in row]
        self.assertEqual(local, [{'name': 'pluto-api', 'version': '0.1.0', 'dependencies': ['serde_json']}])
        manifest = tomllib.loads((ROOT / 'mixins/pluto_native_creative/api/Cargo.toml').read_text())
        self.assertEqual(manifest['package']['name'], local[0]['name'])
        self.assertEqual(set(manifest['dependencies']), set(local[0]['dependencies']))
        runner = (ROOT / 'tests/test_fluid_api.py').read_text()
        self.assertIn('shutil.copytree(source, crate,', runner)
        self.assertIn('tests/fluid-api.Cargo.lock', runner)
        self.assertIn('["cargo", "test", "--offline", "--locked",', runner)

    def test_runner_fetch_is_explicit_and_tests_remain_offline_locked(self):
        source = (ROOT / 'tests/test_inventory_transactions.py').read_text()
        self.assertIn('os.environ.get("PLUTO_TEST_FETCH_LOCKED") == "1"', source)
        self.assertIn('["cargo", "fetch", "--locked",', source)
        self.assertIn('["cargo", "test", "--offline", "--locked",', source)
        self.assertIn('tests/inventory-api.Cargo.lock', source)
        workflow = (ROOT.parent / '.github/workflows/mixins.yml').read_text()
        self.assertIn('PLUTO_TEST_FETCH_LOCKED=1 python3 -m unittest discover -s pluto-lib/tests -v', workflow)
        self.assertNotIn('pluto-lib/tests -p', workflow)

if __name__ == '__main__':
    unittest.main()
