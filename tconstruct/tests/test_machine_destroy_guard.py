"""Static routing checks; real player/world behavior is not exercised."""
from pathlib import Path
import json, sys, unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from piggy_labels import WARNINGS, MACHINE_GUARD_MESSAGES, mark_unavailable
ROOT = Path(__file__).resolve().parents[1] / 'mixins/tconstruct'
class DestroyGuardWiring(unittest.TestCase):
    def test_player_guard_precedes_existing_overwield_spend(self):
        manifest = (ROOT / 'horizon-mixin.toml').read_text()
        entry = manifest.split('id = "source-overwield-harvest-start"', 1)[1].split('[[', 1)[0]
        self.assertIn('native_destroy_block', entry)
        self.assertIn('AFTER_CALL', entry)
        self.assertIn('dispatch_destroy_block', entry)
        self.assertIn('name = "canceled", ty = "&mut bool"', entry)
        helper = (ROOT / 'source-overwield-harvest-start.rs').read_text()
        self.assertLess(helper.index('machine_destroy_guard::protect'), helper.index('tconstruct_overwield_harvest_start(game, *canceled)'))
        self.assertIn('if !*canceled', helper)
        self.assertIn('*canceled = true', helper)
        self.assertIn('__horizon_mixins_tconstruct_smeltery_local_containers()', helper)
        self.assertIn('__horizon_mixins_tconstruct_smeltery_guest_containers()', helper)
        self.assertNotIn('tconstruct:seared_faucet', helper)
        self.assertIn('if let Some(uuid) = game.lan_acting_uuid', helper)
        self.assertIn('game.native_action_bar(uuid, message)', helper)
        self.assertIn('game.titles.set_overlay(message, false)', helper)
        self.assertLess(helper.index('if !*canceled'), helper.index('message.tconstruct.machine_break_guard'))
    def test_closed_projection_proof_is_created_only_after_validation_and_revoked(self):
        close = (ROOT / 'source-builder-close.rs').read_text()
        self.assertIn('if game.remote.is_none()', close)
        self.assertLess(close.index('::close_projection('), close.index('game.give_or_drop(stack)'))
        self.assertIn('if cleared { game.world.chunks.__horizon_mixins_tconstruct_smeltery_closed_projection_mut().insert(pos); }', close)
        guard = (ROOT / 'source-machine-destroy-guard.rs').read_text()
        self.assertIn('closed_projection && page_offset.is_none()', guard)
        self.assertIn('live.len() == super::tconstruct_source_smeltery_page::MENU_SIZE', guard)
        self.assertIn('&& live.iter().all(Option::is_none)', guard)
        self.assertIn('if protect("tconstruct:smeltery", raw) { return false; }', guard)
        self.assertLess(guard.index('if live.len() != expected.len()'), guard.index('live.fill(None)'))
        for filename in ['source-machine-open.rs', 'source-machine-remove.rs',
                         'part-selection-chunk-clear.rs', 'part-selection-chunk-remove.rs']:
            self.assertIn('__horizon_mixins_tconstruct_smeltery_closed_projection_mut()', (ROOT / filename).read_text())
        helper = (ROOT / 'source-overwield-harvest-start.rs').read_text()
        self.assertIn('::live_matches_with_closed_projection(kind, &raw, slots, offset, closed)', helper)
        self.assertIn('::protect(kind, &raw)', helper)
        import tomllib
        manifest = tomllib.loads((ROOT / 'horizon-mixin.toml').read_text())
        state = [row for row in manifest['state'] if row['id'] == 'smeltery-closed-projection']
        self.assertEqual(len(state), 1)
        self.assertEqual(state[0]['ty'], 'std::collections::HashSet<(i32,i32,i32)>')

    def test_all_source_languages_receive_feedback(self):
        self.assertEqual(set(MACHINE_GUARD_MESSAGES), set(WARNINGS))
        for locale in WARNINGS:
            self.assertEqual(mark_unavailable({}, locale)['message.tconstruct.machine_break_guard'], MACHINE_GUARD_MESSAGES[locale])
        language = json.loads((ROOT.parent.parent / 'content/assets/tconstruct/lang/en_us.json').read_text())
        self.assertEqual(language['message.tconstruct.machine_break_guard'], MACHINE_GUARD_MESSAGES['en_us'])
if __name__ == '__main__': unittest.main()
