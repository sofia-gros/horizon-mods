"""Source/package boundaries, not a native crafting-consumption proof."""
import importlib.util
from pathlib import Path
import hashlib
import tempfile
import tomllib
import unittest
import zipfile
ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / 'mixins/pluto_native_creative'
spec = importlib.util.spec_from_file_location('package', ROOT / 'tools/package_native.py')
PACKAGE = importlib.util.module_from_spec(spec)
spec.loader.exec_module(PACKAGE)
class Products(unittest.TestCase):
    def test_public_contract_is_explicit_and_host_independent(self):
        source = (NATIVE / 'inventory-api/src/crafting/products.rs').read_text()
        for value in ('tconstruct:', 'tic_overslime', 'horizon_', 'Mutex', 'OnceLock', 'register(', 'static mut'):
            self.assertNotIn(value, source)
        for value in ('pub trait Preparer', 'pub fn stage_pair', 'pub fn expand', 'MAX_VALUE_DEPTH', 'MAX_PROPOSAL_BYTES', 'StaleInput'):
            self.assertIn(value, source)
        self.assertIn('pub mod products;', (NATIVE / 'inventory-api/src/crafting.rs').read_text())
    def test_module_only_no_automatic_hooks_or_state(self):
        manifest = tomllib.loads((NATIVE / 'horizon-mixin.toml').read_text())
        modules = [m for m in manifest['module'] if m['id'] == 'crafting-products']
        self.assertEqual(len(modules), 1)
        self.assertEqual(modules[0]['name'], 'pluto_products')
        self.assertEqual(modules[0]['file'], 'crates/horizon-client/src/crafting.rs')
        for section in ('injection', 'state'):
            self.assertFalse(any('product' in str(v) for v in manifest.get(section, [])))
        self.assertFalse((NATIVE / 'crafting-products.rs').read_text().startswith('//!'))
    def test_full_clones_not_component_reconstruction(self):
        source = (NATIVE / 'crafting-products.rs').read_text()
        self.assertIn('contains_retained_wire(stack)', source)
        self.assertIn('encode_item_lossless(source)', source)
        self.assertIn('source.clone()', source)
        self.assertIn('encode_item_lossless(&stack)', source)
        self.assertNotIn('item_from_nbt', source)
        self.assertNotIn('from_components', source)
        self.assertNotIn('set_other(', source)
        self.assertIn('d.other.iter().find', source)
        self.assertIn('pluto_nbt_storage::patch_compound', source)
        self.assertIn('v != &updated', source)
    def test_release_is_explicit_value_recheck(self):
        source = (NATIVE / 'crafting-products.rs').read_text()
        self.assertIn('source: self.source.to_vec()', source)
        self.assertIn('inventory_snapshot_matches(&self.source, current)', source)
        self.assertIn('pub fn release_if_unchanged(', source)
        self.assertNotIn('pub pair:', source)
        self.assertNotIn('&mut [Option<ItemStack>]', source)
        self.assertLess(source.index('products::validate(proposal'), source.index('products::stage_pair(proposal'))
        self.assertIn('EmptyRemainderCell', source)
    def test_archive_carries_exact_api_adapter_and_conversion(self):
        with tempfile.TemporaryDirectory() as d:
            archive = Path(d) / 'products.hnmod'
            PACKAGE.build(archive)
            with zipfile.ZipFile(archive) as z:
                for path in ('inventory-api/src/crafting/products.rs', 'crafting-products.rs', 'component-view.rs'):
                    self.assertEqual(z.read('mixins/pluto_native_creative/pluto_native_creative/' + path), (NATIVE / path).read_bytes())
    def test_matching_veto_is_unchanged(self):
        # This service grants no bypass of admission or placement denial.
        expected = {'pluto-lib/mixins/pluto_native_creative/crafting-match-veto.rs': '5d721dbe618c13f29698dea2e4438d6057e7b7d9185cb3322622080fca09fcc2', 'pluto-lib/tools/check_crafting_match_plan.py': 'ceb6e770c1de129e9a64269b568eb27290ceaaa02b70ecf3eba8744195cb70fb', 'pluto-lib/tools/crafting-match-boundaries.json': '90dd56366e9266d08db1fd536547ffee1d3fdb606ea7e67d92cb74e693082aab'}
        for path, digest in expected.items():
            self.assertEqual(hashlib.sha256((ROOT.parent / path).read_bytes()).hexdigest(), digest)
if __name__ == '__main__':
    unittest.main()
