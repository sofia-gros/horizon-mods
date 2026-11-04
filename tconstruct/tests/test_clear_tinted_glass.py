"""Pinned clear tinted glass registration and its source casting recipe."""
import json
import hashlib
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
C = ROOT / 'content'
class ClearTintedGlass(unittest.TestCase):
    def test_source_recipe_and_registration(self):
        with ZipFile(C / 'reference/tconstruct-recipes.zip') as z:
            r = json.loads(z.read('data/tconstruct/recipes/smeltery/casting/amethyst/glass.json'))
        self.assertEqual(r['result'], 'tconstruct:clear_tinted_glass')
        self.assertEqual(r['cast'], {'tag':'forge:glass/colorless'})
        self.assertTrue(r['cast_consumed'])
        self.assertEqual(r['fluid'], {'amount':200, 'tag':'tconstruct:molten_amethyst'})
        self.assertEqual(r['cooling_time'], 97)
        block = json.loads((C / 'data/tconstruct/horizon/block/clear_tinted_glass.json').read_text())
        self.assertEqual(block['template'], 'minecraft:tinted_glass')
        self.assertEqual(block['properties']['hardness'], 0.3)
        tag = json.loads((C / 'reference/tconstruct-tags/data/forge/tags/items/glass/tinted.json').read_text())
        self.assertIn('tconstruct:clear_tinted_glass', tag['values'])
        loot = json.loads((C / 'data/tconstruct/loot_table/blocks/clear_tinted_glass.json').read_text())
        self.assertEqual(loot['pools'][0]['entries'][0]['name'], 'tconstruct:clear_tinted_glass')
        self.assertTrue((C / 'assets/tconstruct/items/clear_tinted_glass.json').is_file())
        texture = C / 'assets/tconstruct/textures/block/clear_tinted_glass'
        self.assertTrue(texture.with_suffix('.png').is_file())
        self.assertEqual({p.stem for p in texture.glob('*.png')}, set('u d l r ud ul ur dl dr lr udl udr ulr dlr udlr'.split()))

    def test_source_texture_git_blobs(self):
        # GitHub tree entries at upstream f4e77d43; Git blob hashes include
        # the "blob <length>\0" header and were checked independently.
        expected = {
            'clear_tinted_glass.png':'5ccdb964681d60bd64cd0647175db15f1a051020',
            'd.png':'8cab253ad36e8d95fb1829c722b39c78331fbdbe', 'dl.png':'75aa87421b18b6eb1f6c0033c33e1e57e891bdec',
            'dlr.png':'92fa3f8030baa5fab75c0ce6a402df8566ca7552', 'dr.png':'2899f885c2c63521f8a8e7ae648d4ae83ee0ae59',
            'l.png':'3718fa73fc741274ed2bf05028cf998dfaa8bc34', 'lr.png':'47ff856d3d18111394539840d05182ff2b0043a7',
            'r.png':'e372a6e201a087e1f4836d656f7fd9a544e9b4f3', 'u.png':'b27f1dcf50e31f04c7371e7a46a793d01e35d58a',
            'ud.png':'84790182c4ef0632fe3fab027d3de1706c9452b3', 'udl.png':'00a118aaa1bb8f1bfe4377552d3a8a7c8c665660',
            'udlr.png':'d420b88076d8e9167c9d49a80053e93dcbf0a979', 'udr.png':'ef5b6768db2c4bc6f0a7156998aa10ef71aef116',
            'ul.png':'dd816e64c65de6ecf8ed8377464613a5efb50148', 'ulr.png':'1954d056ac5a8396969bf73c5fa207629e689278',
            'ur.png':'9429f43f440158b38e40dd2e08193da5dac4502a',
        }
        base = C / 'assets/tconstruct/textures/block'
        for name, sha in expected.items():
            data = ((base / name) if name == 'clear_tinted_glass.png' else (base / 'clear_tinted_glass' / name)).read_bytes()
            blob = b'blob ' + str(len(data)).encode() + b'\0' + data
            self.assertEqual(hashlib.sha1(blob).hexdigest(), sha, name)

if __name__ == '__main__': unittest.main()
