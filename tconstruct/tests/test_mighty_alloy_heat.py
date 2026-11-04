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
        for phrase in ('1,000°C', '1,250°C', '1,500°C', 'not yet been established', 'Vanilla netherite crafting is a separate route'):
            self.assertIn(phrase, text)

    def test_adapted_page_is_packaged(self):
        name = 'assets/tconstruct/book/mighty_smelting/en_us/smeltery/alloying.json'
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'tconstruct.hmod'
            package(out)
            with ZipFile(out) as packed:
                self.assertEqual(json.loads(packed.read(name)), json.loads((ROOT / 'content' / name).read_text()))

if __name__ == '__main__':
    unittest.main()
