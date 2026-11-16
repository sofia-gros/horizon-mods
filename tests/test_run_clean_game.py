"""Reject stale executable evidence and isolate every development launch."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
import subprocess

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
try:
    import run_clean_game as run
finally:
    sys.path.pop(0)


class CleanGameRun(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.prepared = self.root / '.local/clean-port/prepared'
        self.stage = self.prepared / 'applied'
        self.stage.mkdir(parents=True)
        (self.stage / 'source.rs').write_text(newline='\n', data='fixture source')
        self.binary = self.root / '.local/cache/game.exe'
        self.binary.parent.mkdir(parents=True)
        self.binary.write_bytes(b'fixture executable')
        for name in ('contract-check', 'native-check', 'api-check', 'consumer-check', 'game-entry-check'):
            (self.prepared / (name + '.json')).write_text(newline='\n', data=json.dumps({
                'exit': 0, 'source_sha256': run.source_digest(self.stage)}))
        self.proof = {'exit': 0, 'binary_linked': True, 'horizon_revision': run.PIN,
                      'source_sha256': run.source_digest(self.stage),
                      'executable': str(self.binary), 'binary_sha256': run.digest(self.binary)}
        self.save()

    def save(self):
        (self.prepared / 'game-entry-link.json').write_text(newline='\n', data=json.dumps(self.proof))

    def check(self):
        return run.checked_executable(self.prepared, self.root / '.local')

    def test_source_binary_and_failed_prerequisites_refuse(self):
        self.assertEqual(self.check()[0], self.binary.resolve())
        for target in (self.stage / 'source.rs', self.binary):
            original = target.read_bytes()
            target.write_bytes(original + b'changed')
            with self.subTest(target=target), self.assertRaisesRegex(ValueError, 'changed'):
                self.check()
            target.write_bytes(original)
        for name in ('contract-check', 'native-check', 'api-check', 'consumer-check', 'game-entry-check'):
            path = self.prepared / (name + '.json')
            original = path.read_bytes()
            path.write_text(newline='\n', data=json.dumps({'exit': 1}))
            with self.subTest(prerequisite=name), self.assertRaisesRegex(ValueError, 'failed prerequisite'):
                self.check()
            path.write_bytes(original)
        self.proof['horizon_revision'] = 'different-pin'
        self.save()
        with self.assertRaisesRegex(ValueError, 'pinned'):
            self.check()

    def test_matching_external_binary_is_not_a_cache_output(self):
        outside = self.root / 'external.exe'
        outside.write_bytes(self.binary.read_bytes())
        self.proof['executable'] = str(outside)
        self.save()
        with self.assertRaisesRegex(ValueError, 'repository-local'):
            self.check()

    def test_each_launch_uses_fresh_home_and_propagates_failure(self):
        with patch.object(run, 'ROOT', self.root), patch.object(run.subprocess, 'run',
                return_value=subprocess.CompletedProcess([], 7)) as process:
            for _ in range(2):
                self.assertEqual(run.launch(self.binary, self.proof, self.root / 'resources'), 7)
        calls = process.call_args_list
        homes = [Path(c.kwargs['env']['HORIZON_HOME']) for c in calls]
        self.assertNotEqual(*homes)
        for call, home in zip(calls, homes):
            self.assertTrue(home.is_relative_to(self.root / '.local/clean-port'))
            self.assertIn(str(home), call.args[0])
            self.assertEqual(json.loads((home.parent / 'launch.json').read_text())['exit'], 7)

    def test_automation_endpoint_is_inside_fresh_home_only(self):
        with patch.object(run, 'ROOT', self.root), patch.object(run.subprocess, 'run',
                return_value=subprocess.CompletedProcess([], 0)) as process:
            run.launch(self.binary, self.proof, self.root / 'resources', automation=True)
            call = process.call_args
            command = call.args[0]
            self.assertIn('--offscreen', command)
            endpoint = Path(command[command.index('--control-socket') + 1])
            self.assertEqual(endpoint.parent, Path(call.kwargs['env']['HORIZON_HOME']).parent)
            self.assertEqual(endpoint.name, 'control.json')
            run.launch(self.binary, self.proof, self.root / 'resources')
            self.assertNotIn('--offscreen', process.call_args.args[0])
            self.assertNotIn('--control-socket', process.call_args.args[0])


if __name__ == '__main__':
    unittest.main()
