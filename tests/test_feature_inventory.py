import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
import feature_inventory as inv  # noqa: E402


class Inventory(unittest.TestCase):
    def test_every_inventory_file_is_well_formed_and_covers_the_original_package(self):
        for path in sorted((ROOT / 'docs/inventory').glob('*.json')):
            domain, count, open_, problems = inv.report(path)
            self.assertEqual(problems, [], f'{domain}: {problems}')

    def test_a_class_that_is_not_listed_is_a_problem(self):
        data = json.loads((ROOT / 'docs/inventory/world.json').read_text(encoding='utf-8'))
        self.assertTrue(data['classes'])
        if not (inv.JAVA / data['java']).exists():
            self.skipTest('the original source is not checked out')
        removed = dict(data['classes'])
        removed.pop(next(iter(removed)))
        tmp = ROOT / '.local/tmp/inventory-test.json'
        tmp.parent.mkdir(parents=True, exist_ok=True)
        tmp.write_bytes(json.dumps({**data, "classes": removed}).encode("utf-8"))
        self.assertTrue(inv.report(tmp)[3])


if __name__ == '__main__':
    unittest.main()
