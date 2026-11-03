"""Verify pinned item textures, registrations, and tag membership."""
import hashlib
import json
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]
SHAS = {
 'earth': '52c25e3cac17cfa94547db46eac04e460e5781fa',
 'sky': '2c2d567cc11622cab77a21893a1a8bf016ee85ca',
 'ender': '8b94dde411d6eafe43e3551f3f20d6596412b8b0',
 'blood': '5f715d95249e40300ad858d89f7f230558d22d73',
 'ichor': '04a42cb7d072ecd26f0610048c0b39743bf99087',
}
class SlimeSeedsContent(unittest.TestCase):
    def test_registered_items_and_original_textures(self):
        content = ROOT / 'content'
        tag = json.loads((content / 'data/tconstruct/tags/item/slimy_grass_seeds.json').read_text())['values']
        for kind, expected in SHAS.items():
            name = f'{kind}_slime_grass_seeds'
            self.assertIn(f'tconstruct:{name}', tag)
            item = json.loads((content / f'data/tconstruct/horizon/item/{name}.json').read_text())
            self.assertEqual(item['template'], 'minecraft:stick')
            model = json.loads((content / f'assets/tconstruct/models/item/{name}.json').read_text())
            self.assertEqual(model['textures']['layer0'], f'tconstruct:item/{name}')
            data = (content / f'assets/tconstruct/textures/item/{name}.png').read_bytes()
            self.assertEqual(hashlib.sha1(f'blob {len(data)}\0'.encode() + data).hexdigest(), expected)
if __name__ == '__main__':
    unittest.main()
