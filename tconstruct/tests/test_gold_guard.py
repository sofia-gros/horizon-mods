"""Gold Guard source definitions and authoritative attribute/reload ownership."""
import json
from pathlib import Path
import re
import tomllib
import unittest
from unittest.mock import patch
from tempfile import TemporaryDirectory
import run_gold_guard
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'


class GoldGuard(unittest.TestCase):
    def test_runner_rejects_wrong_revision_and_dirty_source(self):
        with patch.object(run_gold_guard.subprocess, 'check_output', return_value='wrong\n'), \
             patch.object(run_gold_guard.subprocess, 'run') as run:
            with self.assertRaises(RuntimeError):
                run_gold_guard.verify_source(Path('/host'), 'pinned')
            run.assert_not_called()
        with patch.object(run_gold_guard.subprocess, 'check_output', return_value='pinned\n'), \
             patch.object(run_gold_guard.subprocess, 'run', side_effect=run_gold_guard.subprocess.CalledProcessError(1, 'git')):
            with self.assertRaises(run_gold_guard.subprocess.CalledProcessError):
                run_gold_guard.verify_source(Path('/host'), 'pinned')

    def test_runner_rejects_dependency_drift(self):
        package = '[[package]]\nname="serde_json"\nversion="1.0.0"\nsource="registry+https://example.test"\nchecksum="abc"\n'
        with TemporaryDirectory() as directory:
            original, generated = (Path(directory) / n for n in ('original.lock', 'fixture.lock'))
            original.write_text(package)
            generated.write_text(package + '[[package]]\nname="local-fixture"\nversion="0.1.0"\n')
            run_gold_guard.verify_lock(original, generated)
            for changed in (package.replace('1.0.0', '1.0.1'), package.replace('abc', 'changed')):
                generated.write_text(changed)
                with self.assertRaises(RuntimeError):
                    run_gold_guard.verify_lock(original, generated)

    def test_exact_source_definitions(self):
        helper = (MIXINS / 'source-gold-guard.rs').read_text()
        with ZipFile(ROOT / 'content/reference/tconstruct-modifiers.zip') as archive:
            for name, variable in [('gold_guard', 'guard'), ('golden', 'golden')]:
                source = json.loads(archive.read(f'data/tconstruct/tinkering/modifiers/{name}.json'))
                fixture = re.search(rf'let {variable}=br#"(.*?)"#;', helper).group(1)
                self.assertEqual(json.loads(fixture), source)



if __name__ == '__main__':
    unittest.main()
