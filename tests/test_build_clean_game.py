"""Build entry sequencing and failure propagation, without compiling the host."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import build_clean_game as build
sys.path.pop(0)


class BuildCleanGame(unittest.TestCase):
    def test_pipeline_stops_at_each_failed_stage_and_shares_cache(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(build, 'ROOT', Path(directory)):
            root = Path(directory)
            output, target = root / '.local/clean-port/new', root / '.local/cache'
            for failure in range(5):
                results = [subprocess.CompletedProcess([], 17 if i == failure else 0) for i in range(4)]
                with self.subTest(failure=failure), patch.object(build.subprocess, 'run', side_effect=results) as run:
                    self.assertEqual(build.build(root / 'reference', output, root / 'mixins', target),
                                     17 if failure < 4 else 0)
                    self.assertEqual(run.call_count, min(failure + 1, 4))
                    first = run.call_args_list[0].args[0]
                    self.assertIn('--check-contracts', first)
                    self.assertIn('--check-api', first)
                    for call in run.call_args_list:
                        self.assertEqual(call.kwargs['env']['CARGO_TARGET_DIR'], str(target.resolve()))
                        self.assertEqual(call.kwargs['env']['CARGO_INCREMENTAL'], '0')
                    if failure == 4:
                        self.assertIn('--link', run.call_args.args[0])

    def test_rejects_reused_or_overlapping_outputs_before_any_stage(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(build, 'ROOT', Path(directory)):
            root = Path(directory)
            output = root / '.local/clean-port/new'
            with patch.object(build.subprocess, 'run') as run:
                for bad_output, bad_target in ((root / 'outside', root / '.local/cache'),
                                               (output, output / 'target'),
                                               (output, root / '.local/clean-port'),
                                               (output, root / 'external-cache')):
                    with self.assertRaises(ValueError):
                        build.build(root, bad_output, root, bad_target)
                output.mkdir(parents=True)
                with self.assertRaises(ValueError):
                    build.build(root, output, root, root / '.local/cache')
                run.assert_not_called()


if __name__ == '__main__':
    unittest.main()
