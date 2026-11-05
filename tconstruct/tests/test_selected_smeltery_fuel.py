"""Runner and transaction wiring; behavioral checks run in the pinned Rust fixture."""
from pathlib import Path
import re
import unittest
ROOT=Path(__file__).resolve().parents[2]
class SelectedSmelteryFuel(unittest.TestCase):
    def test_selected_fuel_checks_follow_safe_staging(self):
        s=(ROOT/'tconstruct/mixins/tconstruct/source-machine-runtime.rs').read_text()
        a=s.index('    let mut fuel_update = None;');b=s.index('    if state.phase == 1 {',a);s=s[a:b]
        self.assertIn('if fuel.temperature == 0 { continue; }',s)
        self.assertIn('if !can_heat && !can_alloy { break; }',s)
        self.assertLess(s.index('tank.take(fuel.fluid.amount)'),s.index('if !can_heat && !can_alloy'))
        self.assertLess(s.index('to_nbt_preserving_fuel'),s.index('if !can_heat && !can_alloy'))
        self.assertNotIn('store::write',s)
        self.assertLess(s.index('if !can_heat && !can_alloy'),s.index('state.fuel = fuel.duration'))
    def test_ci_runs_every_named_case(self):
        runner=(ROOT/'tconstruct/tests/run_selected_fuel_rust.sh').read_text()
        source=(ROOT/'tconstruct/tests/rust/smeltery_selected_fuel.rs').read_text()
        cases=re.findall(r'#\[test\]\s*fn (\w+)\(',source)
        self.assertEqual(len(cases),6)
        for name in cases:self.assertIn(name,runner)
        self.assertIn('cargo test --locked --offline',runner)
        self.assertIn('fixture dependencies differ from the pinned host lock',runner)
        self.assertIn('tconstruct/tests/run_selected_fuel_rust.sh',(ROOT/'.github/workflows/mixins.yml').read_text())
