"""Composite guide previews retain all current fluid and input-part choices."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class MaterialCompositeOptions(unittest.TestCase):
    def test_reader_resolves_all_fluids_and_keeps_live_recipe_gates(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        start = source.index('let supported_types:')
        body = source[start:source.index('for (tool, parts) in &context.tools', start)]
        for text in ('hide_in_book', 'CompositeFluidRecipe::parse(bytes, &conditions)',
                     'recipe.material != id || recipe.input == id',
                     'current.intersection(input)', 'page_parts.contains(*part)',
                     'models.contains_key(part)', 'rest.items.items.contains_key(part.as_str())',
                     'fluid_ids(&recipe.fluid).iter()',
                     'material_composite_options(&parts, &fluids, &recipe.input', 'sidebar.push(entry)'):
            self.assertIn(text, body)
        self.assertNotIn('.first()', body)
        self.assertNotIn('.replacen(', body)

    def test_draw_uses_one_option_for_input_material_icon_and_tooltip(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book-draw.rs').read_text()
        start = source.index('if let Some(sidebar) = panel.get("sidebar")')
        body = source[start:source.index('let Some(groups) = panel.get("groups")', start)]
        self.assertEqual(body.count('material_sidebar_entry(entry, seconds)'), 1)
        for field in ('item', 'material', 'tooltip'):
            self.assertLess(body.index('material_sidebar_entry'), body.index(f'entry.get("{field}")'))
        self.assertIn('("Material", horizon_world::nbt::Tag::String(material.into()))', body)

    def test_regressions_cover_combinations_bounds_and_wire(self):
        helper = (ROOT / 'mixins/tconstruct/source-materials-book-recipe.rs').read_text()
        self.assertIn('composite_options_cycle_every_part_and_fluid_with_input_material', helper)
        self.assertIn('composite_options_validate_bounds_and_preserve_literal_names', helper)
        harness = (ROOT / 'tests/book-wire/src/lib.rs').read_text()
        self.assertIn('composite_input_material_and_all_fluid_choices_survive_book_reply', harness)
