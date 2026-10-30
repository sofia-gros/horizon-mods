"""Check that registered block assets resolve inside the source content pack."""
import json
import unittest
from pathlib import Path

CONTENT = Path(__file__).resolve().parents[1] / 'content'
ASSETS = CONTENT / 'assets/tconstruct'
BLOCKS = CONTENT / 'data/tconstruct/horizon/block'

class BlockRegistrationTests(unittest.TestCase):
    def test_new_blocks_have_resolvable_models_and_drops(self):
        original = {'part_builder', 'tinker_station', 'fake_storage_block',
                    'greenheart_planks', 'skyroot_planks', 'bloodshroom_planks', 'enderbark_planks'}
        seen = set()
        def model(identifier):
            if not identifier.startswith('tconstruct:') or identifier in seen:
                return
            seen.add(identifier)
            path = ASSETS / 'models' / (identifier.split(':', 1)[1] + '.json')
            self.assertTrue(path.is_file(), f'missing model {identifier}')
            data = json.loads(path.read_text())
            if 'parent' in data:
                model(data['parent'])
            for texture in data.get('textures', {}).values():
                if isinstance(texture, str) and texture.startswith('tconstruct:'):
                    path = ASSETS / 'textures' / (texture.split(':', 1)[1] + '.png')
                    self.assertTrue(path.is_file(), f'missing texture {texture}')
        def walk(value):
            if isinstance(value, dict):
                if isinstance(value.get('model'), str): model(value['model'])
                for child in value.values(): walk(child)
            elif isinstance(value, list):
                for child in value: walk(child)
        added = [p for p in BLOCKS.glob('*.json') if p.stem not in original]
        self.assertGreater(len(added), 0)
        for path in added:
            name = path.stem
            with self.subTest(block=name):
                declaration = json.loads(path.read_text())
                self.assertTrue(declaration['template'].startswith('minecraft:'))
                walk(json.loads((ASSETS / 'blockstates' / f'{name}.json').read_text()))
                item = json.loads((ASSETS / 'items' / f'{name}.json').read_text())
                walk(item)
                loot = (CONTENT / 'data/tconstruct/loot_table/blocks' / f'{name}.json').read_text()
                self.assertIn(f'tconstruct:{name}', loot)
                json.loads(loot)

    def test_cobalt_ore_drops_raw_cobalt_or_itself_with_silk_touch(self):
        loot = json.loads((CONTENT / 'data/tconstruct/loot_table/blocks/cobalt_ore.json').read_text())
        children = loot['pools'][0]['entries'][0]['children']
        self.assertEqual(children[0]['name'], 'tconstruct:cobalt_ore')
        self.assertEqual(children[0]['condition'], 'minecraft:tool/can_silk_touch')
        self.assertEqual(children[1]['name'], 'tconstruct:raw_cobalt')
        self.assertTrue(any(modifier.get('type') == 'minecraft:apply_bonus' and modifier.get('enchantment') == 'minecraft:fortune'
                            for modifier in children[1]['modifier']))
        raw = json.loads((CONTENT / 'data/tconstruct/horizon/item/raw_cobalt.json').read_text())
        self.assertEqual(raw['template'], 'minecraft:raw_iron')

if __name__ == '__main__':
    unittest.main()
