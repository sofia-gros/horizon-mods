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
    def test_closed_projection_proof_is_persisted_only_after_validation(self):
        close = (ROOT / 'source-builder-close.rs').read_text()
        self.assertIn('if game.remote.is_none()', close)
        self.assertLess(close.index('::close_projection('), close.index('::replace_outer('))
        self.assertLess(close.index('::replace_outer('), close.index('*live = retired'))
        self.assertLess(close.index('*live = retired'), close.index('game.give_or_drop(stack)'))
        guard = (ROOT / 'source-machine-destroy-guard.rs').read_text()
        self.assertIn('closed_projection && page_offset.is_none()', guard)
        self.assertIn('live.len() == super::tconstruct_source_smeltery_page::MENU_SIZE', guard)
        self.assertIn('&& live.iter().all(Option::is_none)', guard)
        self.assertIn('if protect("tconstruct:smeltery", raw) { return false; }', guard)
        self.assertIn('Some(Tag::List(items)) if items.is_empty()', guard)
        self.assertLess(guard.index('if live.len() != expected.len()'), guard.index('live.fill(None)'))
        opened = (ROOT / 'source-machine-open.rs').read_text()
        self.assertLess(opened.index('::live_matches('), opened.index('::refresh_smeltery('))
        self.assertLess(opened.index('::protect('), opened.index('::refresh_smeltery('))
        self.assertLess(opened.index('::open_outer('), opened.index('*slots ='))
        self.assertLess(opened.index('::replace_outer('), opened.index('*slots ='))
        helper = (ROOT / 'source-overwield-harvest-start.rs').read_text()
        self.assertIn('::live_matches(kind, &raw, slots, offset)', helper)
        self.assertIn('::protect(kind, &raw)', helper)
        runtime = (ROOT / 'source-machine-runtime.rs').read_text()
        self.assertIn('let fresh_smeltery', runtime)
        self.assertIn('&& game.world.block_entities.get(&pos).is_none()', runtime)
        self.assertIn('if fresh_smeltery {', runtime)
        self.assertIn('"tconstruct:smeltery" => 54', runtime)
        self.assertLess(runtime.index('::capture_legacy('), runtime.index('let bounds ='))
        for name in ['source-machine-open.rs', 'source-machine-runtime.rs', 'source-chute-hopper.rs',
                     'source-smeltery-ui.rs', 'source-smeltery-pointer.rs', 'source-smeltery-click.rs']:
            source = (ROOT / name).read_text()
            lines = source.splitlines()
            for index, line in enumerate(lines):
                if ('*live = ' in line or '*slots = ' in line) and '::project' in line:
                    self.assertIn('source_machine_projection_world::record(', lines[index + 1], name)
        clicked = (ROOT / 'source-smeltery-click.rs').read_text()
        self.assertIn('record(&mut game.world, pos, offset);\n            return Break(handled)', clicked)
        self.assertIn('"tconstruct:smeltery" => 54', (ROOT / 'source-machine-decode.rs').read_text())

    def test_all_source_languages_receive_feedback(self):
        self.assertEqual(set(MACHINE_GUARD_MESSAGES), set(WARNINGS))
        for locale in WARNINGS:
            self.assertEqual(mark_unavailable({}, locale)['message.tconstruct.machine_break_guard'], MACHINE_GUARD_MESSAGES[locale])
        language = json.loads((ROOT.parent.parent / 'content/assets/tconstruct/lang/en_us.json').read_text())
        self.assertEqual(language['message.tconstruct.machine_break_guard'], MACHINE_GUARD_MESSAGES['en_us'])
if __name__ == '__main__': unittest.main()
