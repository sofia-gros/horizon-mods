"""Validate the Copper Can RETURN injection's machine-consumed contract."""
from pathlib import Path
import tomllib
import unittest


class CopperCanDisplayContract(unittest.TestCase):
    def test_flowing_water_uses_the_same_tint_and_shared_display_name(self):
        root = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'
        render = (root / 'render-copper-can.rs').read_text()
        tooltip = (root / 'part-name-injection.rs').read_text()
        self.assertIn('tconstruct_source_melter_render::color_tagged(&fluid, tag.as_ref())', render)
        self.assertIn('tconstruct_source_melter_render::sprite(&fluid)', render)
        self.assertIn('tconstruct_source_melter_render::fluid_name(game, &fluid)', tooltip)

    def test_copper_can_uses_source_translation_and_advanced_id(self):
        root = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'
        tooltip = (root / 'part-name-injection.rs').read_text()
        for key in ('item.tconstruct.copper_can.contents', 'item.tconstruct.copper_can.tooltip', 'item.tconstruct.tank.fluid_id'):
            self.assertIn('Text::translate("' + key + '"', tooltip)
        self.assertIn('game.settings.file.bool("advancedItemTooltips")', tooltip)
        self.assertIn('Text::literal(fluid)', tooltip)
        self.assertIn('can_display_empty(stack)', tooltip)
        self.assertNotIn('format!("{name}: 90 mB")', tooltip)

    def test_portable_tank_tooltip_uses_name_and_original_id(self):
        root = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'
        tooltip = (root / 'part-name-injection.rs').read_text()
        self.assertIn('tank_display_fluid(stack,', tooltip)
        self.assertIn('tconstruct_source_melter_render::fluid_name(game, &fluid)', tooltip)
        self.assertIn('Text::translate("item.tconstruct.tank.fluid_id", &[horizon_client::text::Text::literal(fluid)])', tooltip)
        self.assertIn('game.settings.file.bool("advancedItemTooltips")', tooltip)
        self.assertIn('if !hidden_tooltip', tooltip)

    def test_portable_potion_tooltip_uses_complete_metadata(self):
        root = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'
        tooltip = (root / 'part-name-injection.rs').read_text()
        self.assertIn('tank_display_potion(stack,', tooltip)
        self.assertIn('fluid_name_tagged(game, "tconstruct:potion", Some(&tag), true)', tooltip)
        self.assertIn('Text::literal("tconstruct:potion")', tooltip)
        self.assertIn('if !hidden_tooltip', tooltip)

    def test_copper_can_potion_tooltip_is_separate_from_icon(self):
        root = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'
        tooltip = (root / 'part-name-injection.rs').read_text()
        render = (root / 'render-copper-can.rs').read_text()
        self.assertIn('can_display_potion(stack,', tooltip)
        self.assertIn('fluid_name_tagged(game, "tconstruct:potion", Some(&tag), true)', tooltip)
        self.assertIn('can_display_potion_icon(stack, &known)', render)
        self.assertIn('can_display_fluid(stack,', render)

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
