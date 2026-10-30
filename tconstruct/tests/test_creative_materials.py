import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from creative_materials import required_stats, single_material_variants


class MaterialVariants(unittest.TestCase):
    def test_part_and_scaled_material_requirements(self):
        parts = {'modules': [{'type': 'tconstruct:part_stats', 'parts': ['tconstruct:pick_head', {'item': 'tconstruct:tool_handle'}]}]}
        self.assertEqual(required_stats(parts), ['tconstruct:head', 'tconstruct:handle'])
        armor = {'modules': [{'type': 'tconstruct:material_stats', 'stat_types': ['tconstruct:laces', {'stat': 'tconstruct:slime', 'scale': 1.3}]}]}
        self.assertEqual(required_stats(armor), ['tconstruct:laces', 'tconstruct:slime'])

    def test_source_single_material_rule_and_first_fallback(self):
        materials = [('wood', {'head', 'handle'}), ('flint', {'head'}), ('string', {'binding'})]
        self.assertEqual(single_material_variants(['head', 'handle'], materials), [['wood', 'wood'], ['flint', 'wood']])
        self.assertEqual(single_material_variants(['head', 'missing'], materials), [])
