"""Pinned Mantle Honey Bottle input and bundled stock Honey fluid selection."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]


class HoneyBottleTransfer(unittest.TestCase):
    def test_source_empty_transfer_and_bundled_fluid(self):
        source = json.loads((ROOT / 'tests/fixtures/fluid-transfer/honey_bottle_empty.json').read_text())
        self.assertEqual(source, {
            'type': 'mantle:empty_item',
            'conditions': [{'type': 'mantle:tag_filled', 'registry': 'minecraft:fluid', 'tag': 'forge:honey'}],
            'fluid': {'amount': 250, 'tag': 'forge:honey'},
            'input': {'item': 'minecraft:honey_bottle'},
            'result': 'minecraft:glass_bottle',
        })
        with ZipFile(ROOT / 'content/reference/tconstruct-fluid-tags.zip') as archive:
            forge = json.loads(archive.read('data/forge/tags/fluids/honey.json'))
            stock = json.loads(archive.read('data/tconstruct/tags/fluids/honey.json'))
        self.assertEqual(forge['values'], ['#tconstruct:honey'])
        self.assertEqual(stock['values'], ['tconstruct:honey', 'tconstruct:flowing_honey'])


if __name__ == '__main__':
    unittest.main()
