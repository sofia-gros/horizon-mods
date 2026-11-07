"""PR697 source/policy checks, not Rust or actual Game execution."""
import hashlib
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
TC = ROOT / 'tconstruct/mixins/tconstruct'
COMMON = ROOT / 'pluto-lib/mixins/pluto_native_creative'


class RevitalizingIntake(unittest.TestCase):
    def test_native_publication_has_one_shared_owner(self):
        consumer = (TC/'gold-guard-world.rs').read_text()
        shared = (COMMON/'health-attributes.rs').read_text()
        self.assertIn('super::pluto_health_attributes::refresh(game, &wanted);', consumer)
        for forbidden in ('player.set_modifier', 'player.set_health', 'has_modifier('):
            self.assertNotIn(forbidden, consumer)
            self.assertIn(forbidden, shared)
        self.assertNotIn('tconstruct:', shared)
        self.assertNotIn('static ', shared)

    def test_native_batch_precedes_one_nonhealing_clamp(self):
        shared = (COMMON/'health-attributes.rs').read_text()
        self.assertEqual(shared.count('player.set_modifier('), 1)
        self.assertEqual(shared.count('player.set_health('), 1)
        self.assertLess(shared.index('for &(id, wanted)'), shared.index('if changed {'))
        self.assertIn('player.health.health() > maximum', shared)
        self.assertIn('player.set_health(maximum, false)', shared)
        self.assertIn('"add_value", wanted.is_some()', shared)
        self.assertNotIn('applied_modifiers.push', shared)

    def test_invalid_requests_reject_before_native_borrow(self):
        shared = (COMMON/'health-attributes.rs').read_text()
        for part in ('game.remote.is_some()', 'id.is_empty()', '!value.is_finite()', 'previous == id'):
            self.assertLess(shared.index(part), shared.index('game.player.as_mut()'))

    def test_tconstruct_policy_is_not_generalized(self):
        consumer = (TC/'gold-guard-world.rs').read_text()
        for part in ('known_revitalizing', 'tconstruct:modifiable/armor/worn',
                     'slot!=Some(index)', 'is_broken(stack)', 'level<0',
                     'level_with_upgrade(stack,&traits,source::REVITALIZING)?!=level',
                     '0.0f32+2.0f32*level as f32'):
            self.assertIn(part, consumer)
        self.assertLess(consumer.index('let wanted='), consumer.index('pluto_health_attributes::refresh'))

    def test_one_game_private_module_and_no_new_hook(self):
        manifest = tomllib.loads((COMMON/'horizon-mixin.toml').read_text())
        matching = [m for m in manifest['module'] if m.get('id') == 'health-attributes'
                    or m.get('name') == 'pluto_health_attributes' or m.get('source') == 'health-attributes.rs']
        self.assertEqual(matching, [{'id':'health-attributes','file':'crates/horizon-client/src/game.rs',
            'name':'pluto_health_attributes','source':'health-attributes.rs','visibility':'pub(crate)'}])
        for injection in manifest.get('injection', []):
            self.assertNotIn('health-attributes', str(injection))

    def test_actual_shared_source_is_included_by_retained_adapter(self):
        fixture = (ROOT/'tconstruct/tests/rust/gold_guard_runtime.rs').read_text()
        self.assertIn('include!("../../../pluto-lib/mixins/pluto_native_creative/health-attributes.rs")', fixture)
        for name in ('revitalizing_worn_levels_clamp_without_healing_and_restore',
                     'revitalizing_same_refresh_gold_guard_swap_uses_final_maximum',
                     'revitalizing_reload_wrong_slot_upgrade_and_remote_fail_closed',
                     'revitalizing_changed_definition_tag_and_invalid_level_remove_bonus',
                     'shared_health_rejects_invalid_batch_without_partial_publication',
                     'shared_health_batches_swap_clamp_and_noop_without_healing',
                     'shared_health_remote_and_absent_player_are_untouched'):
            self.assertEqual(fixture.count('fn '+name+'('), 1)


if __name__ == '__main__': unittest.main()
