"""Source/package boundaries; behavioral native fixtures are run separately."""
from pathlib import Path
import importlib.util
import tempfile
import tomllib
import unittest
import zipfile
ROOT=Path(__file__).resolve().parents[1]
NATIVE=ROOT/'mixins/pluto_native_creative'
spec=importlib.util.spec_from_file_location('preview_package',ROOT/'tools/package_native.py')
package=importlib.util.module_from_spec(spec);spec.loader.exec_module(package)
class PreviewBoundaries(unittest.TestCase):
    def test_explicit_modules_without_state_or_hooks(self):
        m=tomllib.loads((NATIVE/'horizon-mixin.toml').read_text());modules={x['id']:x for x in m['module']}
        for name,target in [('recipe-book-preview','crates/horizon-client/src/game/recipe_book.rs'),('recipe-book-preview-cells','crates/horizon-client/src/recipe_book.rs')]:
            self.assertEqual(modules[name]['file'],target)
            self.assertFalse((NATIVE/modules[name]['source']).read_text().startswith('//!'))
            for kind in ('injection','state','crate'):self.assertFalse(any(x['id']==name for x in m.get(kind,[])))
    def test_capability_hides_native_types_and_keeps_consumer_policy_external(self):
        api=(NATIVE/'inventory-api/src/recipe_book/preview.rs').read_text()
        self.assertIn('type Selection: Selection;',api)
        for forbidden in ('horizon_','tconstruct','ItemStack','pub struct Game','static REGISTRY'):self.assertNotIn(forbidden,api)
        native=(NATIVE/'recipe-book-preview.rs').read_text()
        for forbidden in ('.send(','.menu_session(','.place_recipe(','.inventory', '.level.recipe_book', 'static ', 'ptr_eq'):self.assertNotIn(forbidden,native)
        self.assertIn('same_item_snapshot',native);self.assertIn('same_ingredient',native);self.assertIn('same_entry(current, &selected.entry)',native)
        self.assertIn('self.0.fill_ghost',''.join(native.split()))
    def test_overlay_is_read_only_and_validates_overrides(self):
        source=(NATIVE/'recipe-book-preview-cells.rs').read_text()
        self.assertIn('book.items_of',source.replace('\n','').replace('    ',''))
        self.assertIn('ingredient_previews',source);self.assertIn('entry.grid_cells(3, 3)',source)
        self.assertNotIn('&mut',source);self.assertNotIn('placements.remove',source)
    def test_actual_archive_contains_exact_api_and_helpers(self):
        with tempfile.TemporaryDirectory() as d:
            output=Path(d)/'preview.hnmod';package.build(output)
            with zipfile.ZipFile(output) as z:
                prefix='mixins/pluto_native_creative/pluto_native_creative/'
                for name in ('recipe-book-preview.rs','recipe-book-preview-cells.rs','inventory-api/src/recipe_book/preview.rs'):
                    self.assertEqual(z.read(prefix+name),(NATIVE/name).read_bytes())
if __name__=='__main__':unittest.main()
