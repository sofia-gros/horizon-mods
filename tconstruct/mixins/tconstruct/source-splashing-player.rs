//! Direct guest-player interaction: the caller already checked packet reach.
use horizon_world::nbt::Tag;

pub(super) fn queue(game: &mut super::Game<'_>, entity: i32) -> bool {
    if game.remote.is_some() { return false; }
    let Some(actor) = game.lan_acting else { return false; };
    let Some(snapshot) = game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_scope().get(&actor).cloned() else { return false; };
    let Some((Some(primary), _, origin, min, max)) = snapshot.iter().find(|(_, id, _, _, _)| *id == entity).copied() else { return false; };
    let Some(stack) = game.held().cloned() else { return false; };
    if !super::tconstruct_source_splashing::right_click(game, &stack) || super::tconstruct_source_melee_shape::intact(&stack).is_none() || game.cooldowns.is_on_cooldown(&stack.item) { return false; }
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return false; };
    let Some(Tag::Int(level)) = custom.get("tic_upgrades").and_then(|upgrades| upgrades.get("tconstruct:splashing")) else { return false; };
    if *level <= 0 { return false; }
    let Some(Tag::Float(bits)) = custom.get("tic_draw_speed") else { return false; };
    let speed = f32::from_bits(*bits);
    if !speed.is_finite() || speed <= 0.0 { return false; }
    let Some(traits) = super::tconstruct_source_material_traits::levels(&game.world, &stack) else { return false; };
    let expanded = super::tconstruct_source_overslime::upgrade(&stack, "tconstruct:expanded") + traits.levels.get("tconstruct:expanded").copied().unwrap_or(0);
    let Ok(expanded) = u32::try_from(expanded) else { return false; };
    let radius = 1.0 + f64::from(expanded);
    if game.armor_stands.iter().any(|stand| !stand.marker && {
        let delta = [stand.position.x - origin[0], stand.position.y - origin[1], stand.position.z - origin[2]];
        delta.iter().map(|component| component * component).sum::<f64>() < radius * radius
    }) { return false; }
    let mut players = vec![primary];
    for (conn, id, position, lower, upper) in snapshot {
        if id == entity { continue; }
        if conn.is_none() {
            if (0..3).map(|axis| (position[axis] - origin[axis]).powi(2)).sum::<f64>() < radius * radius { return false; }
        } else if super::tconstruct_source_splashing::in_range(origin, position, (min, max), (lower, upper), expanded) {
            players.push(conn.unwrap());
        }
    }
    let mut mobs = Vec::new();
    for mob in &game.mobs {
        if !mob.alive() { continue; }
        let position = [mob.position.x, mob.position.y, mob.position.z];
        let bounds = mob.bounds(); let lower = bounds.min_position(); let upper = bounds.max_position();
        if super::tconstruct_source_splashing::in_range(origin, position, (min, max), ([lower.x, lower.y, lower.z], [upper.x, upper.y, upper.z]), expanded) {
            if !super::tconstruct_source_spilling::eligible_mob(mob) { return false; }
            mobs.push(mob.id);
        }
    }
    let count = mobs.len() + players.len();
    if !super::tconstruct_source_splashing::deferred_recipe_supported(game, &stack, *level, count, count <= 2) { return false; }
    let slot = game.held_slot();
    let creative = game.creative;
    let modifier_level = *level;
    game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_pending_mut().insert(actor, (stack.clone(), slot, mobs, players, creative, modifier_level, true));
    game.add_cooldown(stack.item.clone(), (20.0 / speed) as i32);
    true
}
