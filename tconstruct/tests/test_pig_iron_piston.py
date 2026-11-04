"""Verify the Pig Iron preparation hook and cache generation stay connected."""
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class PigIronPistonWiring(unittest.TestCase):
    def test_prepare_and_owned_cache_use_corrected_rows(self):
        manifest = (ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text()
        source = (ROOT / 'mixins/tconstruct/resource-tables.rs').read_text()
        manual = (ROOT / 'mixins/tconstruct/prepare.rs').read_text()
        self.assertIn('name = "tconstruct_pig_iron_piston"', manifest)
        self.assertIn('super::tconstruct_pig_iron_piston::apply(&facts, &mut gameplay)?;', source)
        self.assertLess(source.index('super::tconstruct_pig_iron_piston::apply'), source.index('out.join("gameplay-facts.json.tmp")'))
        self.assertIn('tconstruct-native-creative-v6', source)
        self.assertIn('tconstruct_resource_tables::apply(&out, &content_path)?', manual)

if __name__ == '__main__':
    unittest.main()
