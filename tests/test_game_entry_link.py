"""Exercise input binding and Cargo artifact evidence without compiling the host."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
try:
    import check_game_entry as entry
finally:
    sys.path.pop(0)


class GameEntryLink(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.prepared = self.root / '.local/clean-port/fixture'
        self.stage = self.prepared / 'applied'
        self.target = self.root / '.local/cache'
        game = self.stage / 'crates/native-game'
        (game / 'src').mkdir(parents=True)
        (game / 'src/main.rs').write_bytes(entry.MAIN.encode())
        (game / 'Cargo.toml').write_bytes(entry.MANIFEST.encode())
        (self.stage / 'Cargo.lock').write_text('fixture lock')
        (self.stage / 'texture.bin').write_bytes(b'embedded fixture asset')
        self.product = {'api/src/lib.rs': b'fixture consumer'}
        (self.prepared / 'consumer-check.json').write_text(json.dumps({'consumer_inputs': {
            name: hashlib.sha256(data).hexdigest() for name, data in self.product.items()}}))
        self.receipt = {'exit': 0, 'horizon_revision': entry.PIN,
                        'source_sha256': entry.source_digest(self.stage)}
        self.save_receipt()
        root_patch = patch.object(entry, 'ROOT', self.root)
        root_patch.start(); self.addCleanup(root_patch.stop)
        product_patch = patch.object(entry, 'checked_files', return_value=(self.product,))
        product_patch.start(); self.addCleanup(product_patch.stop)

    def save_receipt(self):
        (self.prepared / 'game-entry-check.json').write_text(json.dumps(self.receipt))

    def cargo_result(self, code=0):
        binary = self.target / 'debug/horizon-client.exe'
        binary.parent.mkdir(parents=True, exist_ok=True)
        binary.write_bytes(b'fixture executable')
        message = {'reason': 'compiler-artifact', 'target': {'name': 'horizon-client', 'kind': ['bin']},
                   'executable': str(binary)}
        return subprocess.CompletedProcess([], code, json.dumps(message)+'\n', 'fixture warning')

    def test_snapshot_rejects_missing_or_unreadable_tree_and_tracks_nested_target(self):
        with self.assertRaisesRegex(ValueError, 'missing or linked'):
            entry.source_digest(self.stage / 'missing')
        def unreadable_tree(*args, **kwargs):
            kwargs['onerror'](PermissionError('fixture access denied'))
            return iter(())
        with patch.object(entry.os, 'walk', side_effect=unreadable_tree):
            with self.assertRaises(PermissionError):
                entry.source_digest(self.stage)
        nested = self.stage / 'assets/target'
        nested.mkdir(parents=True)
        (nested / 'embedded.bin').write_bytes(b'build input')
        self.assertNotEqual(entry.source_digest(self.stage), self.receipt['source_sha256'])

    def test_unchanged_inputs_record_exact_binary_and_refuse_evidence_reuse(self):
        result = self.cargo_result()
        with patch.object(entry.subprocess, 'run', return_value=result) as run:
            self.assertEqual(entry.link(self.prepared, self.target), 0)
            command = run.call_args.args[0]
            self.assertIn('--locked', command)
            self.assertIn('--offline', command)
            self.assertEqual(run.call_args.kwargs['env']['CARGO_INCREMENTAL'], '0')
            proof = json.loads((self.prepared / 'game-entry-link.json').read_text())
            self.assertTrue(proof['binary_linked'])
            self.assertFalse(proof['gameplay_checked'])
            self.assertEqual(proof['binary_sha256'], hashlib.sha256(b'fixture executable').hexdigest())
            with self.assertRaisesRegex(ValueError, 'already exists'):
                entry.link(self.prepared, self.target)
            self.assertEqual(run.call_count, 1)

    def test_stale_missing_or_changed_input_evidence_rejects_before_cargo(self):
        with patch.object(entry.subprocess, 'run') as run:
            for relative in ('Cargo.lock', 'texture.bin', 'crates/native-game/src/main.rs'):
                file = self.stage / relative
                original = file.read_bytes()
                file.write_bytes(original + b'changed')
                with self.subTest(relative=relative), self.assertRaises(ValueError):
                    entry.link(self.prepared, self.target)
                file.write_bytes(original)
            self.receipt.pop('source_sha256')
            self.save_receipt()
            with self.assertRaisesRegex(ValueError, 'lack a snapshot'):
                entry.link(self.prepared, self.target)
            self.receipt.update(source_sha256=entry.source_digest(self.stage), exit=101)
            self.save_receipt()
            with self.assertRaisesRegex(ValueError, 'successful pinned'):
                entry.link(self.prepared, self.target)
            with self.assertRaisesRegex(ValueError, 'repository-local'):
                entry.link(self.prepared, self.root / 'outside-cache')
            run.assert_not_called()

    def test_cargo_failure_and_concurrent_source_change_do_not_certify_binary(self):
        for failure in ('cargo', 'source', 'artifact'):
            with self.subTest(failure=failure):
                result = self.cargo_result(code=101 if failure == 'cargo' else 0)
                if failure == 'artifact':
                    result.stdout = ''
                def compile(*args, **kwargs):
                    if failure == 'source':
                        (self.stage / 'texture.bin').write_bytes(b'concurrent edit')
                    return result
                self.receipt['source_sha256'] = entry.source_digest(self.stage)
                self.save_receipt()
                with patch.object(entry.subprocess, 'run', side_effect=compile):
                    self.assertNotEqual(entry.link(self.prepared, self.target), 0)
                proof_file = self.prepared / 'game-entry-link.json'
                proof = json.loads(proof_file.read_text())
                self.assertFalse(proof['binary_linked'])
                self.assertNotIn('binary_sha256', proof)
                proof_file.unlink()


if __name__ == '__main__':
    unittest.main()
