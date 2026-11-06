"""Source guards only; none execute the proposed Rust or a native host."""
import importlib.util
import os
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('proposal_prepare', HERE / 'prepare.py')
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)


class SourceGuards(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        official = os.environ.get('PLUTO_APPROVED_OFFICIAL')
        if not official:
            raise RuntimeError('PLUTO_APPROVED_OFFICIAL must name the approved clean official checkout')
        cls.original = prepare.source(Path(official))
        cls.changed = prepare.proposed(cls.original)

    def test_patch_is_exact_and_not_a_manifest(self):
        self.assertEqual((HERE / 'UNAPPLIED-player-instance.patch').read_text(),
                         prepare.patch(self.original, self.changed))
        self.assertFalse((HERE / 'horizon-mixin.toml').exists())
        self.assertIn('UNAPPLIED PROTOTYPE', (HERE / 'extension.rs').read_text())

    def test_existing_update_body_is_byte_unchanged(self):
        start = '    pub fn update(&mut self, take: &Self) -> bool {'
        end = '    /// `MobEffectInstance.tickClient`'
        old = self.original[self.original.index(start):self.original.index(end)]
        new = self.changed[self.changed.index(start):self.changed.index(end)]
        self.assertEqual(old, new)

    def test_save_and_remaining_native_code_are_unchanged_and_still_lossy(self):
        self.assertEqual(self.original[self.original.index(prepare.STOP):],
                         self.changed[self.changed.index(prepare.STOP):])
        self.assertIn('pub fn from_saved(saved: &[(String, i32, i32, bool, bool)])', self.changed)
        self.assertIn('pub fn saved(&self) -> Vec<(String, i32, i32, bool, bool)>', self.changed)
        start = self.changed.index('    pub fn saved(&self)')
        end = self.changed.index('    /// ClientboundUpdateMobEffectPacket', start)
        self.assertNotIn('hidden', self.changed[start:end])
        self.assertNotIn('extensions', self.changed[start:end])

    def test_fixture_has_exact_proposed_excerpt_and_no_potion_mock(self):
        generated = prepare.fixture(self.changed)
        expected = self.changed[:self.changed.index(prepare.STOP)]
        expected = expected.replace('use horizon_client_core::potions;\n', '', 1)
        expected = expected.replace('#[path = "effects/extension.rs"]', '#[path = "extension.rs"]', 1)
        self.assertTrue(generated.startswith(expected))
        self.assertEqual(generated.count('pub struct EffectInstance {'), 1)
        self.assertNotIn('mod potions', generated)
        self.assertNotIn('PlayerEffects {', generated)


if __name__ == '__main__':
    unittest.main()
