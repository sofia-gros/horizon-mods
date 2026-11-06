"""Source-only additive facade checks, not Rust/native execution."""
import hashlib
import json
import os
from pathlib import Path
import re
import unittest
ROOT=Path(__file__).resolve().parents[1]
NATIVE=ROOT/'mixins/pluto_native_creative'
# Optional source-only patch review; hashes still pin every lower input.
LOWER=Path(os.environ.get('PLUTO_FACADE_BASE',str(NATIVE)))
META=json.loads((ROOT/'docs/terrain-author-api-source.json').read_text())
START='\n// BEGIN ADDITIVE TERRAIN AUTHOR FACADE\n'
END='// END ADDITIVE TERRAIN AUTHOR FACADE\n'
class SourceChecks(unittest.TestCase):
    def test_only_appended_facade_changes_original_render_helpers(self):
        source=(NATIVE/'rendering.rs').read_text();self.assertEqual(source.count(START),1)
        original,facade=source.split(START);self.assertTrue(facade.endswith(END))
        self.assertEqual(hashlib.sha256(original.encode()).hexdigest(),META['before_sha256']['rendering.rs'])
    def test_same_lower_implementation_manifest_and_type_owner(self):
        for name,digest in META['before_sha256'].items():
            if name!='rendering.rs':self.assertEqual(hashlib.sha256((LOWER/name).read_bytes()).hexdigest(),digest)
        source=(NATIVE/'rendering.rs').read_text().split(START)[1]
        for module in ['pluto_terrain_bindings','pluto_terrain_cell_geometry','pluto_terrain_cells']:
            self.assertIn('pub use horizon_client::mesher::'+module+'::{',source)
        for forbidden in ['Mutex','OnceLock','static REGISTRY','Box<','Vec<','Arc<','impl Drop','pub fn remove','unsafe']:
            self.assertNotIn(forbidden,source)
    def test_all_methods_only_delegate_to_existing_registration(self):
        source=(NATIVE/'rendering.rs').read_text().split(START)[1]
        expected={'register':'implementation::register_materials(&[(group, renderer)])','register_all':'implementation::register_materials(entries)','register_geometry':'implementation::register(group, renderer)','register_geometry_all':'implementation::register_all(entries)'}
        for name,body in expected.items():
            match=re.search(r'pub fn '+name+r'\([^\n]+\) -> Result<\(\), RegisterError> \{\s*([^}]+)\}',source)
            self.assertIsNotNone(match);self.assertEqual(match[1].strip(),body)
        self.assertEqual(len(re.findall(r'pub fn ',source)),4)
    def test_scope_and_preparation_are_not_automatic_or_claimed_proven(self):
        docs=(ROOT/'docs/terrain-author-api.md').read_text()
        for token in ['prepared, not native-compiled','First mesh access freezes','native_standalone_models','origin `[0, 0, 0]`','not simulation/contact','not an ownership token']:
            self.assertIn(token,docs)
        source=(NATIVE/'rendering.rs').read_text().split(START)[1]
        self.assertEqual(source.count('#[test]'),2)
        self.assertNotIn('register("example:',source)
if __name__=='__main__':unittest.main()
