import pathlib, unittest
ROOT = pathlib.Path(__file__).resolve().parents[1]
class FaucetTankWiring(unittest.TestCase):
    def test_stop_and_known_tank_route_are_connected(self):
        source=(ROOT/'mixins/tconstruct/source-machine-runtime.rs').read_text()
        self.assertIn('if faucet.active { return super::tconstruct_source_faucet_tank::stop(game, pos, faucet); }',source)
        self.assertIn('tconstruct_source_faucet_tank::activate(game, pos, source, target, faucet.clone())',source)
        self.assertIn('tconstruct_source_faucet_tank::tick(game, pos, source, target, faucet.clone())',source)
if __name__=='__main__': unittest.main()
