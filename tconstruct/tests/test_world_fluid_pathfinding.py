"""Deferred opt-in world-fluid path policy and packaging contracts."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
FLUIDS = CONTENT / 'data/tconstruct/registered_fluid'
MANIFEST = CONTENT / 'reference/world-fluid-pathfinding-source.json'
COOL = {
    'beetroot_soup', 'earth_slime', 'ender_slime', 'honey', 'ichor',
    'magma', 'meat_soup', 'mushroom_stew', 'rabbit_stew', 'sky_slime', 'venom',
}


class WorldFluidPathfindingTests(unittest.TestCase):
    def test_hot_nodes_and_cool_opt_out_match_source_registration(self):
        declarations = {p.stem: json.loads(p.read_text()) for p in FLUIDS.glob('*.json')}
        self.assertEqual(len(declarations), 70)
        declared = {name for name, data in declarations.items() if 'path_type' in data}
        self.assertEqual(declared, declarations.keys() - COOL)
        self.assertEqual(len(declared), 59)
        for name, data in declarations.items():
            with self.subTest(fluid=name):
                if name in COOL:
                    self.assertNotIn('path_type', data)
                else:
                    self.assertEqual(data['path_type'], 'lava')
        # Heat/path policy is not inferred from burning or vertical flow.
        self.assertIsNone(declarations['liquid_soul']['interaction'].get('burn'))
        self.assertIsNone(declarations['molten_ender']['interaction'].get('burn'))
        self.assertIn('burn', declarations['magma']['interaction'])
        self.assertEqual(declarations['molten_cinderslime']['path_type'], 'lava')
        provenance = json.loads(MANIFEST.read_text())
        self.assertEqual(set(provenance['fluids']), declared)
        self.assertEqual(provenance['path_type'], 'lava')
        self.assertIsNone(provenance['adjacent_path_type'])

    def test_optional_path_policy_survives_native_package(self):
        sys.path.insert(0, str(ROOT / 'tools'))
        try:
            spec = importlib.util.spec_from_file_location('path_package', ROOT / 'tools/package_hmod.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            with tempfile.TemporaryDirectory() as tmp:
                target = Path(tmp) / 'tconstruct.hmod'
                module.package(target)
                with zipfile.ZipFile(target) as archive:
                    for path in FLUIDS.glob('*.json'):
                        packed = json.loads(archive.read(path.relative_to(CONTENT).as_posix()))
                        self.assertEqual(packed, json.loads(path.read_text()))
        finally:
            sys.path.pop(0)


if __name__ == '__main__':
    unittest.main()
