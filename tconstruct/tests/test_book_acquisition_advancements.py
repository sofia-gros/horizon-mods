"""Source-derived acquisition criteria for the two introductory guides."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ADVANCEMENTS = ROOT / 'content/data/tconstruct/advancement'


class BookAcquisitionAdvancements(unittest.TestCase):
    def test_original_inventory_triggers_and_parent(self):
        for path, item, parent in [
            ('tools/materials_and_you.json', 'tconstruct:materials_and_you', None),
            ('smeltery/puny_smelting.json', 'tconstruct:puny_smelting', 'tconstruct:tools/materials_and_you'),
        ]:
            with self.subTest(path=path):
                value = json.loads((ADVANCEMENTS / path).read_text())
                criterion = value['criteria']['crafted_book']
                self.assertEqual(criterion['trigger'], 'minecraft:inventory_changed')
                self.assertEqual(criterion['conditions']['items'], [{'items': [item]}])
                self.assertEqual(value['requirements'], [['crafted_book']])
                self.assertEqual(value.get('parent'), parent)
                self.assertEqual(value['display']['icon']['id'], item)

    def test_root_background_is_included(self):
        root = json.loads((ADVANCEMENTS / 'tools/materials_and_you.json').read_text())
        self.assertEqual(root['display']['background'], 'tconstruct:textures/gui/advancement_background.png')
        self.assertTrue((ROOT / 'content/assets/tconstruct/textures/gui/advancement_background.png').is_file())


if __name__ == '__main__':
    unittest.main()
