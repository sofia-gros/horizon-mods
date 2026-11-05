"""Pin stock Mantle soup emptying and the TConstruct fluid-tag supply."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
SOUPS = ('beetroot_soup', 'mushroom_stew', 'rabbit_stew')

class SoupFluidTransfer(unittest.TestCase):
    def test_pinned_empty_transfers_and_stock_fluid_tags(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-fluid-tags.zip') as archive:
            for soup in SOUPS:
                with self.subTest(soup=soup):
                    source = json.loads((ROOT / f'tests/fixtures/fluid-transfer/{soup}_empty.json').read_text())
                    self.assertEqual(source, {
                        'type': 'mantle:empty_item',
                        'conditions': [{'type': 'mantle:tag_filled', 'registry': 'minecraft:fluid', 'tag': f'forge:{soup}'}],
                        'fluid': {'amount': 250, 'tag': f'forge:{soup}'},
                        'input': {'item': f'minecraft:{soup}'},
                        'result': 'minecraft:bowl',
                    })
                    self.assertEqual(json.loads(archive.read(f'data/forge/tags/fluids/{soup}.json'))['values'], [f'#tconstruct:{soup}'])
                    self.assertEqual(json.loads(archive.read(f'data/tconstruct/tags/fluids/{soup}.json'))['values'], [f'tconstruct:{soup}', f'tconstruct:flowing_{soup}'])

    def test_stock_soups_have_hunger_effect_definitions(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-fluid-effects.zip') as archive:
            for soup in SOUPS:
                with self.subTest(soup=soup):
                    effect = json.loads(archive.read(f'data/tconstruct/tinkering/fluid_effects/{soup}.json'))
                    self.assertEqual(effect['fluid'], {'amount': 50, 'tag': f'forge:{soup}'})
                    self.assertEqual(len(effect['entity_effects']), 1)
                    self.assertEqual(effect['entity_effects'][0]['type'], 'tconstruct:restore_hunger')
                    self.assertEqual(effect['entity_effects'][0]['representative_item'], f'minecraft:{soup}')

if __name__ == '__main__':
    unittest.main()
