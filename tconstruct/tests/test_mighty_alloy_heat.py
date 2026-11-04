"""Check the player-facing heat warning against pinned machine recipes."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package

class MightyAlloyHeat(unittest.TestCase):
    def test_heat_and_limited_acquisition_notice(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as source:
            def recipe(path):
                return json.loads(source.read('data/tconstruct/recipes/smeltery/' + path + '.json'))
            lava = recipe('melting/fuel/lava')
            blaze = recipe('melting/fuel/blaze')
            alloy = recipe('alloys/molten_netherite')
        self.assertEqual((lava['fluid']['fluid'], lava['temperature']), ('minecraft:lava', 1000))
        self.assertEqual((blaze['fluid']['fluid'], blaze['temperature']), ('tconstruct:blazing_blood', 1500))
        self.assertEqual({entry['recipe']['temperature'] for entry in alloy['recipes']}, {1250})
        page = json.loads((ROOT / 'content/assets/tconstruct/book/mighty_smelting/en_us/smeltery/alloying.json').read_text())
        text = ' '.join(part['text'] for part in page['text'])
        for phrase in ('1,000°C', '1,250°C', '1,500°C', 'see the next page', 'Vanilla netherite crafting is separate'):
            self.assertIn(phrase, text)

    def test_stock_barter_notice_matches_bundled_injector(self):
        injector=json.loads((ROOT/'content/data/tconstruct/mantle/loot_injectors/piglin_bartering.json').read_text())
        entry=injector['pools'][0]['entries'][0]
        self.assertEqual((entry['name'],entry['weight']),('tconstruct:scorched_lantern',20))
        fluid,count=entry['functions']
        self.assertEqual(fluid['fluid'],{'amount':50,'fluid':'tconstruct:blazing_blood'})
        self.assertEqual((count['count']['min'],count['count']['max']),(1.0,4.0))
        root=ROOT/'content/assets/tconstruct/book/mighty_smelting'
        section=json.loads((root/'sections/smeltery.json').read_text())
        alloy=next(i for i,p in enumerate(section) if p['name']=='alloying')
        self.assertEqual(section[alloy+1]['data'],'smeltery/blazing_blood_barter.json')
        page=json.loads((root/'en_us/smeltery/blazing_blood_barter.json').read_text())
        text=' '.join(p['text'] for p in page['text'])
        for phrase in ('stock barter table','1–4 Scorched Lanterns','50 mB','original injector','even identical replacement tables disable it', 'Entity and Blaze Head melting are unavailable', 'Live play and LAN are unverified'):
            self.assertIn(phrase,text)

    def test_adapted_page_is_packaged(self):
        name = 'assets/tconstruct/book/mighty_smelting/en_us/smeltery/alloying.json'
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'tconstruct.hmod'
            package(out)
            with ZipFile(out) as packed:
                self.assertEqual(json.loads(packed.read(name)), json.loads((ROOT / 'content' / name).read_text()))
                name='assets/tconstruct/book/mighty_smelting/en_us/smeltery/blazing_blood_barter.json'
                self.assertEqual(json.loads(packed.read(name)), json.loads((ROOT / 'content' / name).read_text()))

if __name__ == '__main__':
    unittest.main()
