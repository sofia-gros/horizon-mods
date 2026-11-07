"""Building-block source boundaries; not actor timing or native attachment proof."""
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / 'mixins/pluto_native_creative'

class ImmersionCurativesBoundary(unittest.TestCase):
    def test_height_kernel_reuses_contact_budget_and_has_no_native_timing(self):
        source = (NATIVE / 'api/src/fluid/contact/immersion.rs').read_text().split('#[cfg(test)]')[0]
        self.assertIn('let (start, end) = range(bounds, limits)?;', source)
        self.assertLess(source.index('range(bounds, limits)?'), source.index('sample(position)?'))
        for expected in ('surface_y >= bounds.min[1]', 'surface_y - bounds.min[1]',
                         'self.unloaded_cells == 0', 'super::checked_height(&cell, position)?'):
            self.assertIn(expected, source)
        for bad in ('Game<', 'EntityRef', 'PlayerMotion', 'Tick', '0.001', 'tconstruct:', 'unsafe '):
            self.assertNotIn(bad, source)

    def test_native_height_view_uses_only_existing_decoder_and_borrowed_inputs(self):
        source = (NATIVE / 'world-fluid-query.rs').read_text().split('#[cfg(test)]')[0]
        start = source.index('    /// Current stepped-cell')
        method = source[start:source.index('\n}', start)]
        self.assertIn('contact::immersion::stepped_surface(p, self.sample(p))', method)
        for bad in ('&mut', 'player', 'lan_', 'take_entity_id', '.block(', '.set(', 'with_level'):
            self.assertNotIn(bad, method)

    def test_curatives_are_owned_immutable_values_and_have_no_implicit_default(self):
        source = (NATIVE / 'api/src/effects.rs').read_text().split('#[cfg(test)]')[0]
        self.assertIn('pub struct Curatives { items: Option<Vec<String>> }', source)
        self.assertIn('Transfer::OrdinaryUpdate => self.clone()', source)
        self.assertIn('Transfer::DetailsCopy => incoming.clone()', source)
        self.assertIn('if self.items.is_some() { Ok(self) } else { Self::explicit(default()) }', source)
        for bad in ('pub items:', 'items_mut', 'static ', 'Mutex', 'entity_id', 'minecraft:milk_bucket',
                    'EffectInstance', 'Game<', 'unsafe ', 'impl Default'):
            self.assertNotIn(bad, source)

    def test_no_tconstruct_consumer_is_activated(self):
        for path in (ROOT.parent / 'tconstruct/mixins/tconstruct').glob('*.rs'):
            text = path.read_text()
            for name in ('contact::immersion', 'pluto_api::effects', '.query_heights('):
                self.assertNotIn(name, text, path.name)

if __name__ == '__main__': unittest.main()
