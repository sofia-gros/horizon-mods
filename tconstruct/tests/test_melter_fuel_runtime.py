"""Static runner/CI contract; behavioral assertions run through the Rust runner."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]

class MelterFuelRuntime(unittest.TestCase):
    def test_production_body_is_extracted_not_reimplemented(self):
        runner = (ROOT / 'tconstruct/tests/run_melter_fuel_rust.sh').read_text()
        runtime = (ROOT / 'tconstruct/mixins/tconstruct/source-machine-runtime.rs').read_text()
        fixture = (ROOT / 'tconstruct/tests/rust/melter_fuel_runtime.rs').read_text()
        for start, end in [('pub(super) fn kind(', 'fn initial('),
                           ('fn same_material(', 'fn lit('),
                           ('pub(super) fn tick_melter(', 'pub(super) fn casting_at(')]:
            self.assertEqual(runtime.count(start), 1)
            self.assertEqual(runtime.count(end), 1)
            self.assertIn(f"section(text,'{start}', '{end}')", runner)
        self.assertNotIn('fn tick_melter(', fixture)
        self.assertIn('env!("MELTER_RUNTIME_EXTRACT")', fixture)
        self.assertIn('cargo test --locked --offline', runner)
        self.assertIn('status --porcelain --untracked-files=no', runner)
        self.assertIn('fixture dependencies differ from the pinned host lock', runner)

    def test_every_named_case_is_required_and_ci_executes_runner(self):
        runner = (ROOT / 'tconstruct/tests/run_melter_fuel_rust.sh').read_text()
        fixture = (ROOT / 'tconstruct/tests/rust/melter_fuel_runtime.rs').read_text()
        names = re.findall(r'#\[test\]\s*fn (\w+)\(', fixture)
        self.assertEqual(len(names), 7)
        for name in names:
            self.assertIn(name, runner)
        workflow = (ROOT / '.github/workflows/mixins.yml').read_text()
        self.assertIn('HORIZON_HOST=../horizon-official tconstruct/tests/run_melter_fuel_rust.sh', workflow)
        self.assertLess(workflow.index('cargo check --locked'), workflow.index('name: Test production Melter fuel driver'))
