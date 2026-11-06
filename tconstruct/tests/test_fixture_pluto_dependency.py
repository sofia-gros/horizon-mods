"""Bounded host fixtures must use the actual Cargo API instance."""
from pathlib import Path
import tempfile
import tomllib
import unittest
from fixture_pluto_dependency import add_inventory_api, inventory_api_identity, verify_fixture_lock

ROOT = Path(__file__).resolve().parents[2]


class FixturePlutoDependency(unittest.TestCase):
    def test_real_package_path_and_exact_local_lock_identity(self):
        with tempfile.TemporaryDirectory(prefix='pluto-fixture-manifest-') as temp:
            manifest = Path(temp) / 'Cargo.toml'
            manifest.write_text('[package]\nname="test-fixture"\nversion="0.1.0"\nedition="2024"\n')
            add_inventory_api(manifest, ROOT)
            actual = tomllib.loads(manifest.read_text())['dependencies']['pluto-inventory-api']['path']
            api = ROOT / 'pluto-lib/mixins/pluto_native_creative/inventory-api'
            self.assertEqual(Path(actual), api.resolve())
            package = tomllib.loads((api / 'Cargo.toml').read_text())['package']
            self.assertEqual(inventory_api_identity(ROOT), (package['name'], package['version'], None, None))

    def test_lock_check_rejects_changed_host_dependency_identity(self):
        with tempfile.TemporaryDirectory(prefix='pluto-fixture-lock-') as temp:
            root = Path(temp); host = root / 'host'; work = root / 'fixture'
            host.mkdir(); work.mkdir()
            manifest = work / 'Cargo.toml'
            manifest.write_text('[package]\nname="bounded-fixture"\nversion="0.1.0"\n')
            host_lock = '[[package]]\nname="host-dependency"\nversion="1.2.3"\nsource="registry+test"\nchecksum="exact"\n'
            (host / 'Cargo.lock').write_text(host_lock)
            own = '\n[[package]]\nname="bounded-fixture"\nversion="0.1.0"\n'
            (work / 'Cargo.lock').write_text(host_lock + own)
            verify_fixture_lock(manifest, host, ROOT)
            (work / 'Cargo.lock').write_text(host_lock.replace('checksum="exact"', 'checksum="changed"') + own)
            with self.assertRaisesRegex(ValueError, 'differ from official host lock'):
                verify_fixture_lock(manifest, host, ROOT)

    def test_fixture_aliases_reexport_one_actual_crate(self):
        for name in ['pluto_storage_fixture.rs', 'inventory_transaction_guards.rs']:
            source = (ROOT / 'tconstruct/tests/rust' / name).read_text()
            self.assertIn('pub use pluto_inventory_api as pluto_inventory;', source)
            self.assertNotIn('inventory-api/src/lib.rs', source)


if __name__ == '__main__':
    unittest.main()
