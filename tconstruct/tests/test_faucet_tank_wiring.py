import pathlib, unittest
ROOT = pathlib.Path(__file__).resolve().parents[1]
class FaucetTankWiring(unittest.TestCase):
    def test_stop_and_known_tank_route_are_connected(self):
        source=(ROOT/'mixins/tconstruct/source-machine-runtime.rs').read_text()
        self.assertIn('if faucet.active { return super::tconstruct_source_faucet_tank::stop(game, pos, faucet); }',source)
        self.assertIn('tconstruct_source_faucet_tank::activate(game, pos, source, target, faucet.clone())',source)
        self.assertIn('tconstruct_source_faucet_tank::tick(game, pos, source, target, faucet.clone())',source)

class LegacyMelterWiring(unittest.TestCase):
    def test_legacy_reservation_and_tick_patch_only_tank(self):
        source=(ROOT/'mixins/tconstruct/source-machine-runtime.rs').read_text()
        legacy=source[source.index('pub(super) fn activate_faucet('):source.index('pub(super) fn tick_casting(')]
        self.assertEqual(legacy.count('patch_melter_tank('), 6)
        self.assertNotIn('melter.to_nbt()',legacy)
        self.assertGreaterEqual(legacy.count('write_machine_updates('),6)
        self.assertIn('let Some(state) = Melter::from_nbt(&raw) else { return; };', legacy)
        self.assertIn('game.world.block_entities.get(&source), Some(BlockEntity::Chest(_))', legacy)
        self.assertIn('} else { None };', legacy)
        missing=legacy.split('let Some((kind, mut casting)) = casting_at(game, target) else {',1)[1]
        self.assertLess(missing.index('is_casting_block(name)'), missing.index('faucet.tick(input, None)'))

if __name__=='__main__': unittest.main()
