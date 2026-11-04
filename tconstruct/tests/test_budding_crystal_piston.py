"""Verify the world crystal reaction wiring and non-dropping source loot."""
import json
import unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
class BuddingCrystalPiston(unittest.TestCase):
    def test_wiring_and_source_loot(self):
        manifest = (ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text()
        resource = (ROOT / 'mixins/tconstruct/resource-tables.rs').read_text()
        self.assertIn('name = "tconstruct_source_budding_piston"', manifest)
        self.assertLess(resource.index('super::tconstruct_source_budding_piston::apply'), resource.index('out.join("gameplay-facts.json.tmp")'))
        self.assertIn('tconstruct-native-creative-v8', resource)
        module = (ROOT / 'mixins/tconstruct/source-budding-piston.rs').read_text()
        harness = (ROOT / 'tests/condition-snapshot/src/lib.rs').read_text()
        self.assertNotIn('budding-piston-prepared-snapshot.json', module)
        self.assertIn('../../fixtures/budding-piston-prepared-snapshot.json', harness)
        for kind in ('earth', 'sky', 'ender', 'ichor'):
            name = f'budding_{kind}_slime_crystal'
            block = json.loads((ROOT / f'content/data/tconstruct/horizon/block/{name}.json').read_text())
            loot = json.loads((ROOT / f'content/data/tconstruct/loot_table/blocks/{name}.json').read_text())
            self.assertEqual(block['template'], 'minecraft:amethyst_block')
            self.assertTrue(block['properties']['random_ticks'])
            self.assertNotIn('pools', loot)
if __name__ == '__main__': unittest.main()
