"""Small preparation checks, without Cargo, Mixins or a generated stage."""
import importlib.util
import json
from pathlib import Path
import re
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('item_name_prepare', HERE / 'prepare_fixture.py')
prepare = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(prepare)

class PreparationTests(unittest.TestCase):
    def test_only_exact_export_revision_marker_may_be_absent_from_git(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); host = root / 'git'; stage = root / 'stage'
            host.mkdir(); stage.mkdir()
            marker = stage / prepare.REVISION_MARKER
            expected = (prepare.PIN + '\n').encode(); marker.write_bytes(expected)
            prepare.verify_official_input(host, stage, marker.name, prepare.sha(expected))
            for wrong in [prepare.PIN.encode(), b'other\n']:
                marker.write_bytes(wrong)
                with self.assertRaises(ValueError):
                    prepare.verify_official_input(host, stage, marker.name, prepare.sha(expected))
            marker.write_bytes(expected)
            with self.assertRaises(ValueError):
                prepare.verify_official_input(host, stage, marker.name, '0' * 64)
            marker.unlink()
            with self.assertRaises(FileNotFoundError):
                prepare.verify_official_input(host, stage, marker.name, prepare.sha(expected))
            with self.assertRaises(FileNotFoundError):
                prepare.verify_official_input(host, stage, 'ordinary-missing.rs', prepare.sha(expected))

    def test_source_boundaries_must_be_unique_and_ordered(self):
        self.assertEqual(prepare.section('a START body END z', 'START', 'END'), 'START body ')
        for text in ('START START END', 'END START', 'START only'):
            with self.assertRaises(ValueError): prepare.section(text, 'START', 'END')

    def test_missing_or_duplicate_applied_module_is_rejected(self):
        with self.assertRaises(ValueError): prepare.final_module('none', 'gate')
        with self.assertRaises(ValueError): prepare.final_module('pub(crate) mod gate {} pub(crate) mod gate {}', 'gate')
        self.assertEqual(prepare.final_module('before\npub(crate) mod gate {}\n', 'gate'), 'pub(crate) mod gate {}\n')

    def test_frozen_inputs_match_source_bytes(self):
        inputs = json.loads((HERE / 'fixture-inputs.json').read_text())
        for path, digest in inputs['source_sha256'].items():
            self.assertEqual(prepare.sha((HERE / path).read_bytes()), digest, path)

    def test_all_test_names_are_exact_and_nonoverlapping(self):
        inputs = json.loads((HERE / 'fixture-inputs.json').read_text())
        names = []
        for group in inputs['result_groups']:
            source = (HERE / group['source']).read_text()
            actual = [group['prefix'] + name for name in re.findall(r'#\[test\]\s*fn ([a-z0-9_]+)', source)]
            self.assertEqual(actual, group['tests'])
            names.extend(actual)
        self.assertEqual(len(names), inputs['expected_tests'])
        self.assertEqual(len(set(names)), len(names))

if __name__ == '__main__': unittest.main()
