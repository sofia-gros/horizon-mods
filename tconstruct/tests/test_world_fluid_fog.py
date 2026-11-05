"""Deferred source fog range, tint and packaged-content contracts."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'


class WorldFluidFogTests(unittest.TestCase):
    def test_source_ranges_and_explicit_colors(self):
        water = {'earth_slime', 'sky_slime', 'ender_slime', 'venom', 'honey'}
        wide = {'liquid_soul', 'molten_diamond', 'molten_emerald', 'molten_amethyst'}
        colors = {
            'molten_queens_slime': 0x478A33, 'molten_ender': 0x105E51,
            'beetroot_soup': 0x84160D, 'mushroom_stew': 0xCD8C6F,
            'rabbit_stew': 0x984A2C, 'meat_soup': 0xE03E35,
            'seared_stone': 0x4F4A47, 'scorched_stone': 0x3E3029,
            'molten_clay': 0x9B6045, 'molten_debris': 0x411E15,
            'molten_chromium': 0xC4B180, 'molten_cadmium': 0x8AB8C8,
            'molten_bendalloy': 0xD6D2C1,
        }
        paths = list((CONTENT / 'data/tconstruct/registered_fluid').glob('*.json'))
        self.assertEqual(len(paths), 70)
        for path in paths:
            with self.subTest(fluid=path.stem):
                visual = json.loads(path.read_text())['visual']
                expected = {'start': -8 if path.stem in water else 0.25,
                            'end': 24 if path.stem in water else 4 if path.stem in wide
                            else 8 if path.stem == 'molten_glass' else 2}
                if path.stem in colors:
                    expected['color'] = colors[path.stem]
                self.assertEqual(visual['fog'], expected)
                if path.stem == 'molten_queens_slime':
                    self.assertEqual(visual['tint'], 0xFFFFFF)

    def test_fog_declarations_survive_packaging(self):
        sys.path.insert(0, str(ROOT / 'tools'))
        try:
            spec = importlib.util.spec_from_file_location('fog_package', ROOT / 'tools/package_hmod.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            with tempfile.TemporaryDirectory() as tmp:
                target = Path(tmp) / 'tconstruct.hmod'
                module.package(target)
                with zipfile.ZipFile(target) as archive:
                    for path in (CONTENT / 'data/tconstruct/registered_fluid').glob('*.json'):
                        packed = json.loads(archive.read(path.relative_to(CONTENT).as_posix()))
                        self.assertEqual(packed['visual']['fog'], json.loads(path.read_text())['visual']['fog'])
        finally:
            sys.path.pop(0)


if __name__ == '__main__':
    unittest.main()
