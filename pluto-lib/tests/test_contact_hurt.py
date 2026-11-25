"""Contact hurt landing checks, not gameplay proof.

An entity-inside callback queues what it does to the entity (fire, an
effect, a hurt) into the contact transport; the queue drains where the
game applies the mob effects — after a hit and on the player's tick.
These checks pin that a queued hurt lands through the generalized hurt
reach in both drains instead of being dropped — living targets through
the game's own hurt call — and that the two Tinkers consumers (the
punji, the knightmetal bite) queue theirs.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / 'mod'
API = ROOT.parent / 'tconstruct' / 'mixins' / 'tconstruct' / 'api' / 'src'


class ContactHurtTests(unittest.TestCase):
    def test_both_drains_land_a_hurt_through_the_games_hurt_call(self):
        for name in ('pluto-mob-effect-apply.rs', 'pluto-mob-effect-apply-tick.rs'):
            source = (BUNDLE / name).read_text(encoding='utf-8')
            self.assertIn('Contact::Hurt(damage_type,amount)', source)
            self.assertIn('pluto_entity_hurt_game::hurt_any(game,entity,amount,&damage_type)', source)
            self.assertNotIn('dropped: no hurt reach', source)
        # The generalized reach (ENT-W9) still lands a living target through
        # the game's own hurt call, with the same arguments the drains used
        # to pass directly: this dimension, the entity, the amount, the
        # damage type and no attacker.
        reach = (BUNDLE / 'pluto-entity-hurt-game.rs').read_text(encoding='utf-8')
        self.assertIn('game.native_hurt_entity(game.dim.id(), entity, amount, damage_type, None)', reach)

    def test_the_transport_still_carries_a_hurt(self):
        source = (BUNDLE / 'pluto-contact-access.rs').read_text(encoding='utf-8')
        self.assertIn('Hurt(String, f32)', source)
        hooks = (BUNDLE / 'api' / 'src' / 'hooks.rs').read_text(encoding='utf-8')
        self.assertIn('Contact::Hurt(damage_type, amount)', hooks)

    def test_the_punji_and_knightmetal_consumers_queue_their_hurts(self):
        gadgets = (API / 'gadgets.rs').read_text(encoding='utf-8')
        self.assertIn('.entity_inside(punji_inside)', gadgets)
        self.assertIn('damage_type: "minecraft:cactus".to_owned()', gadgets)
        cannon = (API / 'fluid_cannon.rs').read_text(encoding='utf-8')
        self.assertIn('behaviour.entity_inside(knightmetal_bite)', cannon)
        self.assertIn('damage_type: "tconstruct:knightmetal".to_owned()', cannon)
        self.assertIn('"knightmetal_block"', cannon)


if __name__ == '__main__':
    unittest.main()
