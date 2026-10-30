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

if __name__ == '__main__':
    unittest.main()
