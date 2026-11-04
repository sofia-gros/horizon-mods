"""Validate the Copper Can RETURN injection's machine-consumed contract."""
from pathlib import Path
import tomllib
import unittest


class CopperCanDisplayContract(unittest.TestCase):
    def test_flowing_water_uses_the_same_tint_and_shared_display_name(self):
        root = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'
        render = (root / 'render-copper-can.rs').read_text()
        tooltip = (root / 'part-name-injection.rs').read_text()
        self.assertIn('tconstruct_source_melter_render::color(&fluid)', render)
        self.assertIn('tconstruct_source_melter_render::sprite(&fluid)', render)
        self.assertIn('tconstruct_source_melter_render::fluid_name(game, &fluid)', tooltip)

    def test_unit_return_hook_supplies_exactly_one_original_value(self):
        manifest = Path(__file__).resolve().parents[1] / 'mixins/tconstruct/horizon-mixin.toml'
        hooks = tomllib.loads(manifest.read_text())['injection']
        hook = next(item for item in hooks if item['id'] == 'copper-can-contents-gui')
        self.assertEqual(hook['kind'], 'RETURN')
        values = [binding for binding in hook['bindings'] if binding['value'] == '$value']
        self.assertEqual(len(values), 1)
        self.assertEqual(values[0]['ty'], '()')


if __name__ == '__main__':
    unittest.main()
