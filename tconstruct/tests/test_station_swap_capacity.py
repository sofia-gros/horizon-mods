"""Pinned definition and authoritative planner wiring checks, not live GUI tests."""
import json
from pathlib import Path
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]

class StationSwapCapacity(unittest.TestCase):
    def test_pinned_pickaxe_and_hammer_require_three_and_four_parts(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-tool-definitions.zip') as archive:
            for name, count in [('pickaxe', 3), ('sledge_hammer', 4)]:
                definition = json.loads(archive.read(f'data/tconstruct/tinkering/tool_definitions/{name}.json'))
                modules = [m for m in definition['modules'] if m['type'] in ('tconstruct:part_stats', 'tconstruct:tool_parts')]
                self.assertEqual(len(modules), 1)
                self.assertEqual(len(modules[0]['parts']), count)
                self.assertEqual(count <= 3, name == 'pickaxe')
                self.assertLessEqual(count, 5)

    def test_guard_precedes_rebuild_and_consumption_plan(self):
        text = (ROOT / 'mixins/tconstruct/source-station-world.rs').read_text()
        swap = text.split('fn swap_plan_with_capacity(', 1)[1].split('pub(super) fn rebuild_material_swap(', 1)[0]
        self.assertIn('BuildingRecipe::swap_parts_fit(definition, input_capacity)?', swap)
        self.assertLess(swap.index('BuildingRecipe::swap_parts_fit'), swap.index('rebuild_material_swap('))
        self.assertLess(swap.index('BuildingRecipe::swap_parts_fit'), swap.index('Some((output, vec!'))
        self.assertIn('swap_plan_with_capacity(world, slots, input_capacity, reason)', text)
        workflow = (ROOT.parent / '.github/workflows/mixins.yml').read_text()
        self.assertIn('tconstruct_source_station_recipes::tests', workflow)
