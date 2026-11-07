from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]
class FluidEffectSelection(unittest.TestCase):
    def test_conditions_precede_ingredient_and_first_match_is_preserved(self):
        source = (ROOT/'mixins/tconstruct/source-fluid-effects.rs').read_text()
        select = source.split('pub(super) fn select(',1)[1].split('pub(super) fn resolve(',1)[0]
        self.assertLess(select.index('conditions(raw)'), select.index('value.get("fluid")'))
        self.assertIn('if condition == Some(false) { continue; }', select)
        self.assertIn('let Some(bytes) = layers.last() else { continue; };', select)
        self.assertIn('condition?;', select)
        self.assertIn('parse_entity(&value).map(Some)', source)
    def test_real_module_and_ci_runner(self):
        fixture = (ROOT/'tests/rust/fluid_effect_selection.rs').read_text()
        self.assertIn('source-fluid-effects.rs',fixture)
        self.assertIn('pluto-lib/mixins/pluto_native_creative/potion-facts.rs',fixture)
        self.assertIn('pub mod pluto_potions;',fixture)
        ci = (ROOT.parent/'.github/workflows/mixins.yml').read_text()
        self.assertIn('tconstruct/tests/run_fluid_effect_selection_rust.sh',ci)
if __name__=='__main__': unittest.main()
