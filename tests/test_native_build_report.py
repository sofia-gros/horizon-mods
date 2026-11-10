"""The probe must not certify a launcher-default or incomplete artifact."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    'native_build_report', Path(__file__).resolve().parents[1] / 'tools/smoke-local/native_build_report.py')
report_gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report_gate)


class NativeBuildReportTests(unittest.TestCase):
    def test_exact_pins_and_complete_mod_inclusion_are_required(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'native-mods.json'
            report = {'horizon': 'reviewed-host', 'horizon_mixins': 'requested-tool',
                      'mods': [{'id': 'library'}, {'id': 'consumer'}], 'left_out': []}

            def check(value):
                path.write_text(json.dumps(value), encoding='utf-8', newline='\n')
                return report_gate.require_report(directory, 'reviewed-host', 'requested-tool',
                                                  {'library', 'consumer'})

            self.assertEqual(check(report), report)
            for change in ({'horizon_mixins': 'launcher-default'}, {'horizon': 'other-host'},
                           {'mods': [{'id': 'library'}]}, {'left_out': ['optional-mod']},
                           {'left_out': None}):
                with self.subTest(change=change), self.assertRaises(ValueError):
                    check(report | change)
            path.unlink()
            with self.assertRaises(FileNotFoundError):
                report_gate.require_report(directory, 'reviewed-host', 'requested-tool', {'library'})
