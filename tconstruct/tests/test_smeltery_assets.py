"""The Smeltery asset snapshot retains source model loaders for conversion."""
import json
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
