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
    def test_projection_writes_validate_before_mutation_and_commit_atomically(self):
        close = (ROOT / 'source-builder-close.rs').read_text()
        self.assertLess(close.index('::close_projection('), close.index('::replace_outer('))
        self.assertLess(close.index('::replace_outer('), close.index('*live = retired'))
        self.assertLess(close.index('*live = retired'), close.index('game.give_or_drop(stack)'))
        opened = (ROOT / 'source-machine-open.rs').read_text()
        self.assertLess(opened.index('::live_matches('), opened.index('::refresh_smeltery('))
        self.assertLess(opened.index('::protect('), opened.index('*slots ='))
        self.assertLess(opened.index('::open_outer('), opened.index('*slots ='))
        runtime = (ROOT / 'source-machine-runtime.rs').read_text()
        self.assertLess(runtime.index('::known('), runtime.index('let bounds ='))
        self.assertIn('"tconstruct:smeltery" => 54', runtime)
        self.assertIn('"tconstruct:smeltery" => 54', (ROOT / 'source-machine-decode.rs').read_text())
        for name in ['source-smeltery-ui.rs', 'source-smeltery-pointer.rs', 'source-smeltery-click.rs', 'source-chute-hopper.rs']:
            source = (ROOT / name).read_text()
            self.assertLess(source.index('::known('), source.index('::write(') if '::write(' in source else source.index('*slots ='), name)
        clicked = (ROOT / 'source-smeltery-click.rs').read_text()
        transaction = clicked.split('let handled = game.menu_ops', 1)[1]
        self.assertLess(transaction.index('*live = expected.clone()'), transaction.index('::write('))
        store = (ROOT / 'source-melter-store.rs').read_text().split('pub(super) fn write_many', 1)[1]
        self.assertLess(store.index('::transition('), store.index('saved.block_entity_sources = sources'))
        self.assertLess(store.index('saved.block_entity_sources = sources'), store.index('*live = view'))
        self.assertIn('if projections.contains_key(pos) { return false; }', store)
        guard = (ROOT / 'source-machine-destroy-guard.rs').read_text()
        self.assertNotIn('snapshot_view', guard)
        self.assertIn('Some(before) => current_projection(before, live, offset, containers)', guard)
        self.assertIn('live.iter().all(Option::is_none) && retired_projection(raw)', guard)
        self.assertIn('raw.get("Items") != Some(&horizon_items::item_nbt::encode_items(live))', guard)

    def test_guest_pickup_owns_rollback_and_filters_wire_before_serialization(self):
        guest = (ROOT / 'source-smeltery-guest-guard.rs').read_text()
        self.assertLess(guest.index('::current_projection('), guest.index('tconstruct_smeltery_guest_pickup::pickup('))
        self.assertLess(guest.index('data.wire.patch().is_some()'), guest.index('tconstruct_smeltery_guest_pickup::pickup('))
        native = (ROOT / 'source-smeltery-guest-pickup.rs').read_text().split('let (drops, changed) = {', 1)[1]
        self.assertLess(native.index('::write('), native.index('return drops'))
        self.assertIn('hub.guests[gi].inventory = previous_inventory;', native)
        self.assertIn('hub.guests[gi].carried = previous_carried;', native)
        self.assertIn('Vec::new()', native)
        self.assertNotIn('Continue(())', native)
        self.assertLess(native.index('no_retained_wire(&items)'), native.index('state.to_nbt()'))
        local = (ROOT / 'source-smeltery-click.rs').read_text()
        self.assertLess(local.index('data.wire.patch().is_some()'), local.index('state.to_nbt()'))
        self.assertIn('no_retained_wire(view)', local)
        self.assertIn('no_retained_wire(items)', (ROOT / 'source-smeltery-guest-write.rs').read_text())
        self.assertIn('no_retained_wire(original_hopper)', (ROOT / 'source-chute-hopper.rs').read_text())
        self.assertIn('item.stack.data().is_some_and(|data| data.wire.patch().is_some())', (ROOT / 'source-machine-runtime.rs').read_text())

    def test_all_source_languages_receive_feedback(self):
        self.assertEqual(set(MACHINE_GUARD_MESSAGES), set(WARNINGS))
        for locale in WARNINGS:
            self.assertEqual(mark_unavailable({}, locale)['message.tconstruct.machine_break_guard'], MACHINE_GUARD_MESSAGES[locale])
        language = json.loads((ROOT.parent.parent / 'content/assets/tconstruct/lang/en_us.json').read_text())
        self.assertEqual(language['message.tconstruct.machine_break_guard'], MACHINE_GUARD_MESSAGES['en_us'])
if __name__ == '__main__': unittest.main()
