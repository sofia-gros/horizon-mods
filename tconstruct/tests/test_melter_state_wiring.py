import pathlib, unittest
ROOT = pathlib.Path(__file__).resolve().parents[1]
class MelterPreservationWiring(unittest.TestCase):
    def test_tick_uses_lossless_patch_for_machine_and_fuel(self):
        source=(ROOT/'mixins/tconstruct/source-machine-runtime.rs').read_text()
        tick=source.split('pub(super) fn tick_melter',1)[1].split('pub(super) fn casting_at',1)[0]
        self.assertIn('melter.to_nbt_preserving(&saved)',tick)
        self.assertIn('Tank::drain_fuel_preserving(&raw_tank, capacity, fuel.fluid.amount)',tick)
if __name__=='__main__': unittest.main()
