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
    def test_lantern_adapter_is_local_and_stages_before_live_slots(self):
        code = P.read_text()
        route = code[code.index('fn tank_to_casting_tank('):code.index('enum CastingInput')]
        self.assertIn('tanks::Source::Tank("tconstruct:lantern", tank, raw)', route)
        self.assertIn('Tank::from_nbt(&raw, 50)', route)
        self.assertIn('data.wire.patch().is_some()', route)
        self.assertLess(route.index('store::write_many('), route.index('slots[0] = output.slots[0].clone()'))
    def test_cannon_source_rechecks_live_item_after_destination_processing(self):
        code = P.read_text()
        route = code[code.index('fn tank_to_casting_tank('):code.index('enum CastingInput')]
        self.assertIn('CastingInput::Cannon(state, raw)', route)
        self.assertIn('cannon_source && !cannon_at(game, source).is_some_and', route)
        self.assertIn('guarded_item_source && output.slots', route)
        self.assertLess(route.index('process_casting_tank(game, &mut output)'), route.index('cannon_source && !cannon_at'))
        self.assertLess(route.index('cannon_source && !cannon_at'), route.index('store::write_many('))
        adapter = code[code.index('enum CastingInput'):code.index('fn source_process_snapshot_valid(')]
        self.assertIn('patch_melter_tank(raw, &state.tank)', adapter)
if __name__ == '__main__': unittest.main()
