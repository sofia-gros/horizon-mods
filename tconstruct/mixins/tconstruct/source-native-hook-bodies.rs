// Bodies of native-hook adapters live outside the host hook-point file.
use super::{GameLevel, Ticks, BlockPos, BlockStateId, LevelAccess};

pub(super) fn tconstruct_casting_comparator_present(level: &crate::game::GameLevel<'_>, state: horizon_world::registry::BlockStateId, output: &mut bool) {
    if matches!(&*level.facts.get(state).name, "tconstruct:seared_casting_tank" | "tconstruct:seared_fluid_cannon") { *output = true; }
}

pub(super) fn tconstruct_overwield_harvest_start(game: &mut crate::game::Game<'_>, canceled: bool) {
    if canceled { return; }
    use horizon_world::nbt::Tag;
    let actor = game.lan_acting;
    let active = game.world.chunks.__horizon_mixins_tconstruct_area_active().contains(&actor);
    if active && game.world.chunks.__horizon_mixins_tconstruct_area_overwield_spent().contains(&actor) { return; }
    let Some(stack) = game.inventory[game.selected].as_ref() else { return; };
    if crate::game::tconstruct_source_ranged_wear::is_broken(stack) { return; }
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return; };
    if !matches!(custom.get("tic_max_damage"), Some(Tag::Int(max)) if *max > 0 && stack.damage < *max as u32) { return; }
    let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|result| result.as_ref().ok()) else { return; };
    let Some(broad_tag) = tags.get("tconstruct:modifiable/broad") else { return; };
    let Some(harvest_tag) = tags.get("tconstruct:modifiable/harvest") else { return; };
    let broad = broad_tag.contains(stack.item.as_ref());
    let harvest = harvest_tag.contains(stack.item.as_ref());
    if !broad && !harvest { return; }
    let Some(Tag::List(traits)) = custom.get("tic_mining_traits") else { return; };
    let level = traits.iter().find_map(|entry| match (entry.get("id"), entry.get("level")) {
        (Some(Tag::String(id)), Some(Tag::Int(level))) if id.to_string_lossy() == "tconstruct:overwield" => Some((*level).max(0) as u32),
        _ => None,
    }).unwrap_or(0);
    if level == 0 { return; }
    if let Some(held) = game.inventory[game.selected].as_mut() {
        crate::game::tconstruct_source_overslime::spend(held, level.saturating_mul(if broad { 5 } else { 1 }));
    }
    if active { game.world.chunks.__horizon_mixins_tconstruct_area_overwield_spent_mut().insert(actor); }
}
