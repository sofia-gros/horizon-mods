use super::*;
use std::ops::ControlFlow::{self, Break, Continue};

pub(super) fn is_crop(game: &Game<'_>, pos: [i32; 3]) -> bool {
    let name = &game.res.facts().get(game.world.block(pos[0], pos[1], pos[2])).name;
    game.res.items().block_in_tag("tconstruct:source_effective/tconstruct/harvestable/crops", name)
        && !game.res.items().block_in_tag("tconstruct:source_effective/tconstruct/harvestable/interact", name)
}


pub(super) fn harvest(game: &mut Game<'_>, hit: Option<&Hit>) -> ControlFlow<Result<bool>> {
    if game.remote.is_some() || game.keys.contains(&KeyCode::ShiftLeft) { return Continue(()); }
    let Some(hit) = hit else { return Continue(()); };
    let Some(stack) = game.held().cloned() else { return Continue(()); };
    if !matches!(stack.item.as_ref(), "tconstruct:kama" | "tconstruct:scythe") || !is_crop(game, hit.pos) { return Continue(()); }
    let Some(max) = tconstruct_source_transform_logic::maximum(&stack) else { return Continue(()); };
    if stack.damage >= max || !game.may_build() { return Continue(()); }
    if game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().is_none() { tconstruct_recipe_reload::capture(game); }
    let Some(seeds) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
        .and_then(|tags| tags.as_ref().ok()).and_then(|tags| tags.get("tconstruct:seeds")).cloned() else { return Continue(()); };
    let actor = game.lan_acting;
    let blocked = match actor {
        Some(conn) => match game.world.chunks.__horizon_mixins_tconstruct_transform_guest_hits().get(&conn) {
            Some((target, blocked)) if *target == hit.pos => blocked.clone(),
            _ => return Break(Ok(false)),
        },
        None => Default::default(),
    };
    let positions = if stack.item.as_ref() == "tconstruct:scythe" {
        let Some(area) = tconstruct_source_transform_geometry::box_area(hit.pos, hit.face, game.yaw, 2) else { return Break(Ok(false)); };
        std::iter::once(hit.pos).chain(area).collect::<Vec<_>>()
    } else { vec![hit.pos] };
    let result = (|| -> Result<bool> {
        for pos in positions {
            if !game.held().is_some_and(|held| held.item == stack.item && held.damage < max) { break; }
            if blocked.contains(&(pos[0],pos[2])) || !is_crop(game, pos) { continue; }
            let state = game.world.block(pos[0],pos[1],pos[2]);
            let Some(reset) = tconstruct_source_crop_age::mature_reset(game.pack.assets.registry, state) else { continue; };
            let mut drops = game.block_loot_at(pos, state, Some(&stack));
            if tconstruct_source_crop_loot::consume_seed(&mut drops, &seeds) {
                game.level_set(pos, reset)?;
            } else {
                let pos = horizon_core::position::BlockPos::new(pos[0],pos[1],pos[2]);
                game.with_level(|behaviours, level, ticks| behaviours.destroy_block(level, ticks, pos, false));
                game.process_level_events()?;
            }
            for drop in drops.into_iter().filter(|item| item.count > 0) { game.pop_resource(pos, drop); }
            if !game.creative {
                let loss = game.held().map(|held| game.durability_loss(held, 1)).unwrap_or(0);
                if let Some(held) = game.held_mut().as_mut().filter(|held| held.item == stack.item) { tconstruct_tx::fixed_damage(held, loss); }
            }
        }
        game.swing_arm();
        Ok(true)
    })();
    if let Some(conn) = actor { game.world.chunks.__horizon_mixins_tconstruct_transform_guest_hits_mut().remove(&conn); }
    Break(result)
}
