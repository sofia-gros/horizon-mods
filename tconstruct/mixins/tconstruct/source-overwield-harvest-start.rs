// Run after native cancellation handlers and before actual block removal.
// The metadata guard precedes Overwield's spend for this break.
fn tconstruct_overwield_harvest_start(game: &mut crate::game::Game<'_>, pos: [i32; 3], state: horizon_world::registry::BlockStateId, canceled: &mut bool) {
    if !*canceled {
        let name = game.res.facts().get(state).name.clone();
        let kind = match &*name {
            "tconstruct:seared_melter" => Some("tconstruct:melter"),
            "tconstruct:smeltery_controller" | "tconstruct:foundry_controller" => Some("tconstruct:smeltery"),
            _ => None,
        };
        if let Some(kind) = kind {
            let at = (pos[0], pos[1], pos[2]);
            let pending_containers = kind == "tconstruct:smeltery" && (
                game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers().get(&at)
                    .is_some_and(|(slots, _)| slots.iter().any(Option::is_some))
                || game.world.chunks.__horizon_mixins_tconstruct_smeltery_guest_containers().values()
                    .any(|(owner, slots, _)| *owner == at && slots.iter().any(Option::is_some)));
            if pending_containers { *canceled = true; }
            if !game.world.chunks.contains_key(&(pos[0] >> 4, pos[2] >> 4)) {
                *canceled = true;
            } else if let Some(raw) = super::tconstruct_source_melter_store::read_outer(&game.world, at) {
                let offset = game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages().get(&at).map(|page| page.1);
                let safe_live = match game.world.block_entities.get(&at) {
                    Some(horizon_client::containers::BlockEntity::Chest(slots)) =>
                        super::tconstruct_source_machine_destroy_guard::live_matches(kind, &raw, slots, offset),
                    _ => false,
                };
                if !safe_live || super::tconstruct_source_machine_destroy_guard::protect(kind, &raw) { *canceled = true; }
            } else if game.world.block_entities.get(&at).is_some_and(|entity| match entity {
                horizon_client::containers::BlockEntity::Chest(slots) => slots.iter().any(Option::is_some),
                _ => true,
            }) { *canceled = true; }
        }
        if *canceled {
            let message = horizon_client::text::Text::translate("message.tconstruct.machine_break_guard", &[]);
            if let Some(uuid) = game.lan_acting_uuid { let _ = game.native_action_bar(uuid, message); }
            else { game.titles.set_overlay(message, false); }
        }
    }
    super::tconstruct_native_hook_bodies::tconstruct_overwield_harvest_start(game, *canceled);
}
