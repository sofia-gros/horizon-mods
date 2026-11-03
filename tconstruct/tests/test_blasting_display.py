"""Check both UI paths and execute the shared Blasting formatter."""
import json
import pathlib
import subprocess
import shutil
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]

class BlastingDisplay(unittest.TestCase):
    def test_wiring_and_formatter(self):
        station = (ROOT / 'mixins/tconstruct/station-info-ui.rs').read_text()
        tooltip = (ROOT / 'mixins/tconstruct/part-name-injection.rs').read_text()
        self.assertIn('"tconstruct:blasting" => 20', station)
        self.assertIn('("blasting", 20)', tooltip)
        self.assertIn('tconstruct_station_info_ui::blasting_label', tooltip)
        function = station[station.index('pub(super) fn blasting_label'):]
        function = function.replace('pub(super) fn', 'fn', 1)
        harness = function + '''\nfn main() {
          for (amount, expected) in [(1, "Blasting I: 1 / 20"), (19, "Blasting I: 19 / 20"), (20, "Blasting I"), (21, "Blasting II: 1 / 20"), (39, "Blasting II: 19 / 20"), (40, "Blasting II"), (60, "Blasting III"), (80, "Blasting IV"), (99, "Blasting V: 19 / 20"), (100, "Blasting V")] {
            assert_eq!(blasting_label("Blasting", amount).as_deref(), Some(expected));
          }
          assert_eq!(blasting_label("Blasting", 0), None);
          assert_eq!(blasting_label("Blasting", -1), None);
        }\n'''
        if not shutil.which("rustc"):
            self.skipTest("rustc is required to execute the formatter")
        with tempfile.TemporaryDirectory() as tmp:
            src = pathlib.Path(tmp) / 'test.rs'
            src.write_text(harness)
            subprocess.run(['rustc', '--edition=2021', str(src), '-o', str(pathlib.Path(tmp) / 'test')], check=True)
            subprocess.run([str(pathlib.Path(tmp) / 'test')], check=True)
        names = json.loads((ROOT / 'content/assets/tconstruct/lang/en_us.json').read_text())
        self.assertEqual(names['modifier.tconstruct.blasting'], 'Blasting')
        self.assertTrue(names['modifier.tconstruct.blasting.description'])

if __name__ == '__main__': unittest.main()
