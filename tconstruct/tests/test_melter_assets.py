"""Check that the retained source assets contain the melter workflow."""
import json
import unittest
import zipfile
from pathlib import Path

ARCHIVE = Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-melter-assets.zip'


class MelterAssets(unittest.TestCase):
    def test_all_models_and_textures_referenced_by_the_setup_are_present(self):
        with zipfile.ZipFile(ARCHIVE) as z:
            files = set(z.namelist())
            for block in ('seared_melter', 'seared_heater', 'seared_faucet', 'seared_table', 'seared_basin'):
                self.assertIn(f'assets/tconstruct/blockstates/{block}.json', files)
                self.assertIn(f'assets/tconstruct/models/item/{block}.json', files)
            for path in files:
                if not path.endswith('.json') or '/textures/' in path:
                    continue
                value = json.loads(z.read(path))
                def visit(node):
                    if isinstance(node, dict):
                        for key, value in node.items():
                            if isinstance(value, str) and value.startswith('tconstruct:'):
                                ident = value.removeprefix('tconstruct:')
                                if key in ('parent', 'model'):
                                    self.assertIn(f'assets/tconstruct/models/{ident}.json', files)
                                elif '/models/' in path and key not in ('loader', 'render_type') and ident.startswith(('block/', 'item/')):
                                    self.assertIn(f'assets/tconstruct/textures/{ident}.png', files)
                            else:
                                visit(value)
                    elif isinstance(node, list):
                        for child in node:
                            visit(child)
                visit(value)
            source = json.loads(z.read('assets/tconstruct/models/block/template/half_tank.json'))
            self.assertEqual(source['loader'], 'tconstruct:tank')
            self.assertIn('assets/tconstruct/textures/gui/melter.png', files)
            fluids = json.loads(z.read('assets/tconstruct/melter-fluid-textures.json'))
            colors = json.loads(z.read('assets/tconstruct/melter-fluid-colors.json'))
            for fluid in ('tconstruct:molten_copper', 'tconstruct:molten_glass', 'tconstruct:molten_diamond', 'tconstruct:seared_stone', 'tconstruct:earth_slime'):
                self.assertIn(fluid, fluids)
                self.assertIn('assets/tconstruct/' + fluids[fluid], files)
            self.assertEqual(colors['tconstruct:seared_stone'], 0xff4f4a47)
            self.assertTrue(set(colors) <= set(fluids))


if __name__ == '__main__':
    unittest.main()
