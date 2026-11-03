"""Keep the root's renderer template and cullface-bearing model inheritance."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'content'

class EnderbarkRootRender(unittest.TestCase):
    def test_template_and_model_inherit_mangrove_root_visibility(self):
        block = json.loads((ROOT / 'data/tconstruct/horizon/block/enderbark_roots.json').read_text())
        self.assertEqual(block['template'], 'minecraft:mangrove_roots')
        state = json.loads((ROOT / 'assets/tconstruct/blockstates/enderbark_roots.json').read_text())
        model_id = state['variants']['']['model']
        self.assertEqual(model_id, 'tconstruct:block/wood/enderbark/roots/empty')
        model = json.loads((ROOT / 'assets/tconstruct/models/block/wood/enderbark/roots/empty.json').read_text())
        self.assertEqual(model['parent'], 'minecraft:block/mangrove_roots')
        self.assertEqual(model['render_type'], 'minecraft:cutout')

if __name__ == '__main__':
    unittest.main()
