"""Exact-source oracle and package boundary checks, not a rendering test."""
import importlib.util
import json
from pathlib import Path
import tempfile
import tomllib
import unittest
ROOT=Path(__file__).resolve().parents[1];NATIVE=ROOT/'mixins/pluto_native_creative'
spec=importlib.util.spec_from_file_location('terrain_oracle',ROOT/'tools/prepare_terrain_geometry_oracle.py');oracle=importlib.util.module_from_spec(spec);spec.loader.exec_module(oracle)
class TerrainGeometryChecks(unittest.TestCase):
    def test_only_explicit_generic_module_no_hook_state_or_crate(self):
        manifest=tomllib.loads((NATIVE/'horizon-mixin.toml').read_text())
        modules=[m for m in manifest['module'] if m['id']=='terrain-cell-geometry']
        self.assertEqual(modules,[{'id':'terrain-cell-geometry','file':'crates/horizon-client-render/src/mesher.rs','name':'pluto_terrain_cell_geometry','source':'terrain-cell-geometry.rs','visibility':'pub'}])
        for key in ['injection','state','crate']:
            self.assertFalse(any(x['id'].startswith('terrain-cell-geometry') for x in manifest.get(key,[])))
    def test_support_module_has_no_inner_file_attributes(self):
        source=(NATIVE/'terrain-cell-geometry.rs').read_text()
        self.assertNotRegex(source,r'(?m)^\s*(?:#!\s*\[|//!|/\*!)')
    def test_runtime_geometry_uses_fixed_values_only(self):
        source=(NATIVE/'terrain-cell-geometry.rs').read_text().split('#[cfg(test)]')[0]
        for forbidden in ['Vec<','Vec::','Box<','Box::','unsafe','FluidKind','pluto_api','Mutex','RwLock','SectionMesh','TerrainVertex']:
            self.assertNotIn(forbidden,source)
        self.assertIn('[Quad; 12]',source);self.assertIn('Result<Batch, Error>',source)
        self.assertIn('MIT License',source)
    def test_oracle_extraction_rejects_wrong_or_ambiguous_source(self):
        with self.assertRaises(ValueError):oracle.extract('not the official source')
        with self.assertRaises(ValueError):oracle.unique('repeat repeat','repeat')
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);p=root/oracle.SOURCE;p.parent.mkdir(parents=True);p.write_text('wrong')
            with self.assertRaises(ValueError):oracle.prepare(root,root/'out')
    def test_actual_package_contains_exact_geometry_module(self):
        spec=importlib.util.spec_from_file_location('geometry_package',ROOT/'tools/package_native.py');package=importlib.util.module_from_spec(spec);spec.loader.exec_module(package)
        _,files=package._bundle(json.loads((ROOT/'horizon-native.json').read_text()))
        self.assertEqual(files['pluto_native_creative/terrain-cell-geometry.rs'],(NATIVE/'terrain-cell-geometry.rs').read_bytes())
if __name__=='__main__':unittest.main()
