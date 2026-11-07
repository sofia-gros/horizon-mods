"""Faucet evidence must be pinned and cannot pass without boundary tests."""
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from fixture_pluto_dependency import run_locked_fixture

ROOT = Path(__file__).resolve().parent


class FaucetFixtureIdentity(unittest.TestCase):
    def test_all_three_runners_use_real_api_and_official_lock_guard(self):
        for name in ('faucet_tank_casting', 'faucet_tagged_local', 'faucet_smeltery_tank'):
            source = (ROOT / f'run_{name}.py').read_text()
            self.assertIn('add_inventory_api(', source)
            self.assertIn('lock_fixture_to_host(', source)
            self.assertIn('run_locked_fixture(', source)
            self.assertNotIn('generate-lockfile', source)

    def test_required_cases_cannot_be_missing_filtered_or_failed(self):
        for output in ('test result: ok. 0 passed\n', 'test route ... ignored\n', 'test route ... FAILED\n'):
            with patch('subprocess.run', return_value=subprocess.CompletedProcess([], 0, output)), patch('builtins.print'):
                with self.assertRaisesRegex(ValueError, 'Required native fixture'):
                    run_locked_fixture(Path('/tmp/fixture/Cargo.toml'), ['route'])
        with patch('subprocess.run', return_value=subprocess.CompletedProcess([], 0, 'test route ... ok\n')) as run, patch('builtins.print'):
            run_locked_fixture(Path('/tmp/fixture/Cargo.toml'), ['route'])
            self.assertEqual(run.call_args.args[0][:4], ['cargo', 'test', '--locked', '--offline'])

    def test_cargo_failure_remains_failure_even_if_a_case_passed(self):
        with patch('subprocess.run', return_value=subprocess.CompletedProcess([], 101, 'test route ... ok\n')), patch('builtins.print'):
            with self.assertRaises(subprocess.CalledProcessError):
                run_locked_fixture(Path('/tmp/fixture/Cargo.toml'), ['route'])


if __name__ == '__main__':
    unittest.main()
