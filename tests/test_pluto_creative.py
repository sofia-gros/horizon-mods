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
