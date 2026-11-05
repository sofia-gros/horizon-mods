"""A bounded construction plan replaces neither the source viewer nor gameplay."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
BOOK = ROOT / 'content/assets/tconstruct/book/mighty_smelting'


class MightyStructureDiagram(unittest.TestCase):
    def test_pinned_structure_identity_and_adapted_instructions(self):
        raw = (ROOT / 'tests/fixtures/mighty_structure_source.json').read_bytes()
        self.assertEqual(hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest(),
                         'e854189e31eb5eed0d529ea5d61224304aa5a122')
        self.assertEqual(json.loads(raw)['data'], 'tconstruct:book/structures/smeltery.nbt')
        section = json.loads((BOOK / 'sections/smeltery.json').read_text())
        route = next(page for page in section if page['name'] == 'structure')
        self.assertEqual((route['type'], route['data']), ('mantle:structure', 'smeltery/multiblock.json'))
        page = json.loads((BOOK / 'en_us/smeltery/multiblock.json').read_text())
        self.assertEqual(page['structure_diagram'], 'small_smeltery')
        text = ' '.join(part['text'] for part in page['text'])
        for phrase in ('Static top views; north is up', 'two Seared Bricks', 'Face the controller north',
                       '1,080 mB', 'fuel separately', 'not mandatory for formation', 'separate additions',
                       'not the original animated structure viewer or a Foundry plan', 'current structure tags',
                       'Loaded chunks', 'have not been verified'):
            self.assertIn(phrase, text)

    def test_reader_requires_live_block_tags_and_known_icons_before_showing_plan(self):
        source = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        body = source[source.index('let structure_diagram ='):source.index('let mut graphic = structure_diagram')]
        for text in ('mighty_smeltery', 'Some("structure")', 'Some("small_smeltery")',
                     '__horizon_mixins_tconstruct_resolved_block_tags()', 'rest.items.items.contains_key(id)',
                     'small_smeltery_spec(tags,', 'else { continue; }'):
            self.assertIn(text, body)
        self.assertIn('graphic.get("small_smeltery").is_some() { 80.0 }', source)
        self.assertIn('remaining = Some(chunks.into_iter().skip(1)', source)
        draw = (ROOT / 'mixins/tconstruct/source-materials-book-draw.rs').read_text()
        start = draw.index('if spec.get("small_smeltery")')
        body = draw[start:draw.index('if spec.get("modifier_progress")', start)]
        for text in ('small_smeltery_layers()', 'Floor (Y=0)', 'Walls (Y=1)', 'Keep the interior empty',
                     'No block is required here', 'ItemStack::new(id, 1)', 'item_tooltip = Some(stack)',
                     'top + 116.0', '.take(6)'):
            self.assertIn(text, body)

    def test_production_detector_and_wire_cases_share_the_drawn_cells(self):
        harness = (ROOT / 'tests/book-wire/src/lib.rs').read_text()
        for text in ('source-smeltery-structure.rs', 'drawn_small_smeltery_layers_match_the_production_detector',
                     'let layers = recipe::small_smeltery_layers()',
                     'smeltery_structure::discover([1,1,0], [0,-1], accepts)', 'Some(([0,0,0], [2,1,2], 1080))'):
            self.assertIn(text, harness)
        helper = (ROOT / 'mixins/tconstruct/source-materials-book-recipe.rs').read_text()
        self.assertIn('small_smeltery_uses_current_tags_and_all_registered_parts', helper)
