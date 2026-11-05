from pathlib import Path
import unittest

P = Path(__file__).resolve().parents[1] / 'mixins/tconstruct/source-machine-runtime.rs'
class Wiring(unittest.TestCase):
    def test_network_guards_cover_packet_and_legacy_sources(self):
        code = P.read_text()
        tick = code[code.index('pub(super) fn tick_faucet('):code.index('pub(super) fn tick_casting(')]
        self.assertLess(tick.index('!faucet_network_tag_safe(&faucet.pending)'), tick.index('tank_to_casting_tank('))
        self.assertIn('!faucet_network_tag_safe(input) || !faucet_network_tag_safe(&state.tank) || !faucet_network_tag_safe(&faucet.pending)', tick)
        old = code[code.index('fn tick_smeltery_faucet('):code.index('/// Start or stop a source faucet.')]
        self.assertIn('!faucet_network_tag_safe(&fluid) || !faucet_network_tag_safe(&state.tank) || !faucet_network_tag_safe(&faucet.pending)', old)
        for body in (tick, old):
            for field in ('game.remote.is_some()', 'game.lan.is_some()', 'game.lan_acting_uuid.is_some()'):
                self.assertIn(field, body)
if __name__ == '__main__': unittest.main()
