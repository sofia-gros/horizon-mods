import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from creative_blocks import add_block_items

class CreativeBlocksTests(unittest.TestCase):
    def test_registered_block_items_only_and_stable_order(self):
        tabs = [{'id': 'tconstruct:blocks', 'items': []}]
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name, item in [('zinc', True), ('airlike', False), ('amber', True)]:
                (root / (name + '.json')).write_text(json.dumps({'template': 'minecraft:stone', 'item': item}))
            result = add_block_items(tabs, root)
        self.assertEqual(result[0]['items'], ['{count:1,id:"tconstruct:amber"}', '{count:1,id:"tconstruct:zinc"}'])
        self.assertEqual(tabs[0]['items'], [])

    def test_group_covers_current_block_items(self):
        tabs = json.loads((ROOT / 'content/creative-tabs.json').read_text())
        root = ROOT / 'content/data/tconstruct/horizon/block'
        group = next(t for t in add_block_items(tabs, root) if t['id'] == 'tconstruct:blocks')
        self.assertEqual(len(group['items']), sum(json.loads(p.read_text()).get('item', True) for p in root.glob('*.json')))
        self.assertIn('{count:1,id:"tconstruct:seared_bricks"}', group['items'])

if __name__ == '__main__':
    unittest.main()
