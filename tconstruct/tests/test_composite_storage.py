"""Check fixed Forge storage layers against their source inputs."""
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from flatten_composite_storage import flatten

class CompositeStorageTests(unittest.TestCase):
    def test_source_layers_match_registered_geometry(self):
        ref = ROOT / 'content/reference/tconstruct-composite-storage'
        inset = json.loads((ref / 'cube_all_14.json').read_text())
        for name in ('slimesteel', 'cinderslime', 'knightslime'):
            with self.subTest(name=name):
                source = json.loads((ref / f'{name}.json').read_text())
                actual = json.loads((ROOT / f'content/assets/tconstruct/models/block/storage/{name}.json').read_text())
                self.assertEqual(actual, flatten(source, inset))
                self.assertEqual([len(element['faces']) for element in actual['elements']], [6, 6])
                self.assertEqual(actual['elements'][0]['from'], [1, 1, 1])
                self.assertEqual(actual['elements'][1]['from'], [0, 0, 0])

    def test_other_composites_are_rejected(self):
        with self.assertRaises(ValueError):
            flatten({'loader': 'forge:composite', 'children': {}}, {})
