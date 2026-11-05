"""The conservative Haste trait gate stays wired to source data and actor state."""
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class LocalArmorHaste(unittest.TestCase):
    def test_pinned_definition_is_exact_source(self):
        source = (ROOT / 'mixins/tconstruct/source-pure-rules.rs').read_bytes()
        literal = source.split(b'const PINNED_HASTE: &[u8] = br#"', 1)[1].split(b'"#;', 1)[0]
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-modifiers.zip') as archive:
            self.assertEqual(literal, archive.read('data/tconstruct/tinkering/modifiers/haste.json'))
    def test_context_uses_local_actor_and_current_world_snapshots(self):
        source = (ROOT / 'mixins/tconstruct/source-mining-context-shared.rs').read_text()
        for field in ('game.armor[1]', 'game.remote.is_some()', 'tconstruct_source_haste_wire::matching_level',
                      '__horizon_mixins_tconstruct_resolved_item_tags()',
                      '__horizon_mixins_tconstruct_modifier_resource_snapshot()',
                      'tconstruct_source_material_traits::levels(&game.world, stack)'):
            self.assertIn(field, source)
        self.assertIn('unwrap_or(0)', source)
        self.assertLess(source.index('conditional_with_water('), source.rindex('finish_mining_speed('))
        self.assertIn('return speed * armor_multiplier', source)
        self.assertIn('Some(facts.explosion_resistance)', source)
        self.assertLess(source.index('environment_multiplier('), source.index('conditional_with_water('))
        self.assertIn('player.is_eye_in_fluid(horizon_client::motion::WATER)', source)
        self.assertIn('minecraft:aqua_affinity', source)
        self.assertIn('effects.amplifier("mining_fatigue")', source)
        self.assertIn('player.ground.on_ground', source)
        self.assertIn('player.fluids.was_eye_in_water', source)
        sender = (ROOT / 'mixins/tconstruct/source-haste-wire-send.rs').read_text()
        receiver = (ROOT / 'mixins/tconstruct/source-haste-wire-receive.rs').read_text()
        self.assertIn('guest.armor[1]', sender)
        self.assertIn('guest.conn, guest.placed_at', sender)
        self.assertIn('C::UpdateTags(_)', receiver)

if __name__ == '__main__':
    unittest.main()
