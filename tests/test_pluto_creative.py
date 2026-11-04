import unittest
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'pluto-lib'))
from pluto_lib.creative import register_item_groups


class CreativeTabs(unittest.TestCase):
    def test_add_preserves_order_components_and_input(self):
        source = {'tabs': [{'id': 'minecraft:hotbar', 'type': 'HOTBAR', 'row': 'TOP', 'column': 5}]}
        stacks = ['{count:1,id:"example:first"}', '{count:1,id:"example:second",components:{"minecraft:custom_data":{Material:"example:wood"}}}']
        mod = {'id': 'example:tools', 'title': 'Example', 'icon': stacks[0], 'row': 'TOP', 'column': 5, 'items': stacks}
        output = register_item_groups(source, [mod])
        self.assertEqual(len(source['tabs']), 1)
        self.assertEqual(output['tabs'][-1]['items'], stacks)
        self.assertTrue(output['tabs'][-1]['visible'])

    def test_collision_is_rejected(self):
        tab = {'id': 'example:tools', 'title': 'Example', 'icon': 'x', 'row': 'TOP', 'column': 0, 'items': []}
        with self.assertRaises(ValueError):
            register_item_groups({'tabs': [{'id': 'minecraft:building', 'row': 'TOP', 'column': 0}]}, [tab])

class CreativePagedTabs(unittest.TestCase):
    def test_auto_page_preserves_existing_and_avoids_collisions(self):
        existing = {'tabs': [
            {'id':'minecraft:blocks','row':'TOP','column':0},
            {'id':'other:one','row':'TOP','column':0,'page':1},
            {'id':'other:two','row':'TOP','column':1,'page':1},
        ]}
        group = {'id':'tconstruct:world','title':'World','icon':'x',
                 'row':'TOP','column':0,'page':1,'auto_place':True,'items':[]}
        result = register_item_groups(existing, [group])
        self.assertEqual(result['tabs'][:3], existing['tabs'])
        self.assertEqual((result['tabs'][-1]['page'],result['tabs'][-1]['row'],result['tabs'][-1]['column']), (1,'TOP',2))
        self.assertNotIn('auto_place', result['tabs'][-1])
    def test_same_coordinate_different_page_is_valid(self):
        source = {'tabs':[{'id':'minecraft:blocks','row':'TOP','column':0}]}
        tab = {'id':'x:y','title':'World','icon':'x','row':'TOP','column':0,'page':1,'items':[]}
        self.assertEqual(register_item_groups(source,[tab])['tabs'][-1]['page'],1)
    def test_common_column_collision_is_global(self):
        source = {'tabs':[{'id':'minecraft:search','row':'TOP','column':6}]}
        tab = {'id':'x:y','title':'World','icon':'x','row':'TOP','column':6,'page':1,'items':[]}
        with self.assertRaises(ValueError): register_item_groups(source,[tab])
