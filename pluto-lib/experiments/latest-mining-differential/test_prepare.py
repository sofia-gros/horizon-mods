"""Renderer validation tests only; synthetic text never becomes native proof."""
import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

P = Path(__file__).resolve().with_name('prepare_sources.py')
S = importlib.util.spec_from_file_location('mining_prepare', P)
M = importlib.util.module_from_spec(S)
S.loader.exec_module(M)


class RendererTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.data = {
            'official/crates/horizon-client/src/game/hand.rs': b'speed / hardness / if harvest { 30.0 } else { 100.0 }',
            'official/crates/horizon-client/src/game.rs': b'm.progress >= 1.0 && (self.lan_acting.is_none() || m.ticks == 1)',
            'official/crates/horizon-client/src/game/lan_actions.rs': b'Some(p) if p < 1.0 => {}\nif done.is_some() {\n                    self.guest_destroy(host, conn, pos);',
            M.MOD_ROOT + 'source-pure-rules.rs': b'pub(super) fn boost() {}\n#[cfg(test)]',
            M.MOD_ROOT + 'source-mining-math.rs': b'// unchanged math',
            M.MOD_ROOT + 'source-mining-rules.rs': b'// unchanged rules',
            'pluto-lib/mixins/pluto_tool_rules/rules.rs': b'// unchanged tool',
        }
        self.regression = b'// only renderer test input'
        self.seal()

    def seal(self):
        (self.root / 'inputs.json').write_text(json.dumps({
            'source_sha256': {p: M.sha(b) for p, b in self.data.items()},
            'regression_sha256': M.sha(self.regression),
        }))

    def run_render(self, data=None, regression=None):
        with patch.object(M, 'HERE', self.root):
            return M.render(self.data if data is None else data,
                self.regression if regression is None else regression)

    def test_sources_are_copied_without_policy_changes(self):
        out = self.run_render()
        self.assertEqual(len(out), 7)
        self.assertEqual(out['mining_rules.rs'], self.data[M.MOD_ROOT + 'source-mining-rules.rs'])
        self.assertEqual(out['mining_math.rs'], self.data[M.MOD_ROOT + 'source-mining-math.rs'])
        self.assertEqual(out['tool_rules.rs'], self.data['pluto-lib/mixins/pluto_tool_rules/rules.rs'])
        self.assertIn(b'not Game implementations', out['official_fragments.rs'])

    def test_missing_input_rejected(self):
        data = dict(self.data); data.pop(next(iter(data)))
        with self.assertRaisesRegex(ValueError, 'file-set'): self.run_render(data=data)

    def test_extra_input_rejected(self):
        with self.assertRaisesRegex(ValueError, 'file-set'): self.run_render(data={**self.data, 'extra': b''})

    def test_drift_rejected_before_extraction(self):
        data = dict(self.data); key = next(iter(data)); data[key] += b' drift'
        with self.assertRaisesRegex(ValueError, 'byte drift'): self.run_render(data=data)

    def test_regression_drift_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Regression'): self.run_render(regression=b'changed')

    def test_ambiguous_fragment_rejected_even_if_hash_matches(self):
        key = 'official/crates/horizon-client/src/game/hand.rs'
        self.data[key] *= 2; self.seal()
        with self.assertRaisesRegex(ValueError, 'one exact'): self.run_render()


if __name__ == '__main__':
    unittest.main()
