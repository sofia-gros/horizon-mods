"""Pinned TConstruct cluster and bud loot behavior in the native data."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'content/data'
LOOT = ROOT / 'tconstruct/loot_table/blocks'


def load(name):
    return json.loads((LOOT / f'{name}.json').read_text())


class GeodeClusterDrops(unittest.TestCase):
    def test_four_slime_and_cobalt_clusters(self):
        for block, item in [(f'{kind}_slime_crystal_cluster', f'{kind}_slime_crystal')
                            for kind in ('earth', 'sky', 'ender', 'ichor')] + [('cobalt_cluster', 'cobalt_shard')]:
            with self.subTest(block=block):
                value = load(block)
                self.assertEqual(value['random_sequence'], f'tconstruct:blocks/{block}')
                pool = value['pools'][0]
                self.assertEqual(pool['rolls'], 1)
                silk, alternate = pool['entries'][0]['children']
                self.assertEqual((silk['name'], silk['condition']), (f'tconstruct:{block}', 'minecraft:tool/can_silk_touch'))
                rich, normal = alternate['children']
                self.assertEqual(rich['condition'], {'type':'minecraft:match_tool', 'predicate':{'items':'#minecraft:cluster_max_harvestables'}})
                self.assertEqual(rich['name'], f'tconstruct:{item}')
                self.assertEqual(rich['modifier'], [{'type':'minecraft:set_count', 'count':4},
                    {'type':'minecraft:apply_bonus', 'enchantment':'minecraft:fortune', 'formula':'minecraft:ore_drops'}])
                self.assertEqual(normal['name'], f'tconstruct:{item}')
                self.assertEqual(normal['modifier'], [{'type':'minecraft:set_count', 'count':2}, {'type':'minecraft:explosion_decay'}])

    def test_buds_drop_self_only_with_silk_touch(self):
        for color in ('earth', 'sky', 'ender', 'ichor'):
            for size in ('small', 'medium', 'large'):
                name = f'{size}_{color}_slime_crystal_bud'
                with self.subTest(name=name):
                    value = load(name)
                    self.assertEqual(value['random_sequence'], f'tconstruct:blocks/{name}')
                    self.assertEqual(value['pools'], [{'condition':'minecraft:tool/can_silk_touch',
                        'entries':[{'type':'minecraft:item', 'name':f'tconstruct:{name}'}], 'rolls':1}])

    def test_source_tool_tag_additions(self):
        value = json.loads((ROOT / 'minecraft/tags/item/cluster_max_harvestables.json').read_text())
        self.assertEqual(value['values'], ['tconstruct:pickaxe', 'tconstruct:sledge_hammer',
            'tconstruct:vein_hammer', 'tconstruct:war_pick'])


if __name__ == '__main__':
    unittest.main()
