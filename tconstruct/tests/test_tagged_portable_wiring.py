from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'mixins' / 'tconstruct'

class TaggedPortableWiring(unittest.TestCase):
    def test_only_local_tagged_dispatch(self):
        code = (ROOT / 'source-machine-runtime.rs').read_text()
        self.assertIn('game.lan.is_none() && game.lan_acting_uuid.is_none()', code)
        self.assertIn('allow_color.then(|| super::tconstruct_source_melter_world::tagged_portable_casting_stack', code)
        self.assertIn('(Some(_), Some(_)) => None', code)
        world = (ROOT / 'source-melter-world.rs').read_text()
        self.assertIn('catalog.portable_tank_filling(basin, &stack.item, fluid, free', world)
        self.assertIn('fill_portable_tagged(stack, tank, fluid, tag, recipe.fluid.amount)', world)

if __name__ == '__main__':
    unittest.main()
