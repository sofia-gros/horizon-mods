"""Shared decoder guards both modern and legacy Faucet dispatch paths."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'


class FaucetMetadataGuard(unittest.TestCase):
    def test_dispatch_decodes_before_stop_or_transfer(self):
        source = (ROOT / 'source-machine-runtime.rs').read_text()
        activate = source.split('pub(super) fn activate_faucet(', 1)[1].split('pub(super) fn tick_faucet(', 1)[0]
        tick = source.split('pub(super) fn tick_faucet(', 1)[1].split('pub(super) fn tick_casting(', 1)[0]
        self.assertLess(activate.index('Faucet::from_nbt(&tag)'), activate.index('if faucet.active'))
        self.assertLess(activate.index('Faucet::from_nbt(&tag)'), activate.index('faucet_neighbors('))
        self.assertLess(tick.index('Faucet::from_nbt(&tag)'), tick.index('faucet_neighbors('))
        self.assertIn('else { return false; };', activate)
        self.assertIn('else { return; };', tick)

    def test_faucet_decoder_preserves_native_extensions_but_rejects_source_fields(self):
        source = (ROOT / 'source-melter-state.rs').read_text()
        body = source.split('impl Faucet {', 1)[1].split('/// Distinguish a recognized', 1)[0]
        for field in ('"state"', '"drained"', '"render_fluid"', '"lastRedstone"', '"FluidName"', '"Amount"', '"Tag"'):
            self.assertIn(field, body)
        self.assertIn('pending.to_nbt_preserving(raw_pending).as_ref() != Some(raw_pending)', body)
        self.assertIn('original: Some(tag.clone())', body)


if __name__ == '__main__':
    unittest.main()
