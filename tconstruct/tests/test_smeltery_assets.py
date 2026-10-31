"""The Smeltery asset snapshot retains source model loaders for conversion."""
import json
import sys
import tempfile
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]


class SmelteryAssets(unittest.TestCase):
    def test_controller_tank_and_io_source_models_are_preserved(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-smeltery-assets.zip') as archive:
            for block in ('smeltery_controller', 'seared_drain', 'seared_chute', 'seared_duct',
                          'seared_ladder', 'seared_fuel_tank', 'seared_fuel_gauge',
                          'seared_ingot_tank', 'seared_ingot_gauge', 'seared_tinted_glass'):
                self.assertIn('assets/tconstruct/blockstates/' + block + '.json', archive.namelist())
            tank = json.loads(archive.read('assets/tconstruct/models/block/template/tank.json'))
            controller = json.loads(archive.read('assets/tconstruct/models/block/template/controller.json'))
            self.assertEqual(tank['loader'], 'tconstruct:tank')
            self.assertEqual(controller['loader'], 'mantle:retextured')
            self.assertIn('assets/tconstruct/textures/gui/heating_structure.png', archive.namelist())
            for suffix in ('u', 'd', 'l', 'r', 'ud', 'ul', 'ur', 'dl', 'dr', 'lr', 'udl', 'udr', 'ulr', 'dlr', 'udlr'):
                self.assertIn(f'assets/tconstruct/textures/block/smeltery/tinted_glass/{suffix}.png', archive.namelist())
                self.assertIn(f'assets/tconstruct/textures/block/foundry/tinted_glass/{suffix}.png', archive.namelist())
            for block in ('foundry_controller', 'scorched_drain', 'scorched_chute', 'scorched_duct',
                          'scorched_ladder', 'scorched_fuel_tank', 'scorched_fuel_gauge',
                          'scorched_ingot_tank', 'scorched_ingot_gauge', 'scorched_tinted_glass'):
                self.assertIn('assets/tconstruct/blockstates/' + block + '.json', archive.namelist())

    def test_static_shell_generation_is_reproducible(self):
        sys.path.insert(0, str(ROOT / 'tools'))
        try:
            from generate_smeltery_assets import generate
            with tempfile.TemporaryDirectory() as tmp:
                out = Path(tmp)
                generate(out)
                first = {p.relative_to(out): p.read_bytes() for p in out.rglob('*') if p.is_file()}
                generate(out)
                second = {p.relative_to(out): p.read_bytes() for p in out.rglob('*') if p.is_file()}
                self.assertEqual(first, second)
                for block in ('smeltery_controller', 'seared_drain', 'seared_fuel_tank',
                              'foundry_controller', 'scorched_drain', 'scorched_fuel_tank'):
                    self.assertIn(Path(f'data/tconstruct/horizon/block/{block}.json'), first)
                    self.assertIn(Path(f'assets/tconstruct/blockstates/{block}.json'), first)
        finally:
            sys.path.pop(0)
