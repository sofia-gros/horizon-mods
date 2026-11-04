fn tconstruct_crossbow_use(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    if &*stack.item != "tconstruct:crossbow" {
        return Continue(());
    }
    if crate::game::tconstruct_source_ranged_wear::is_broken(stack) { return Break(true); }
    if game.using.is_some() {
        return Break(true);
    }
    if stack.data().is_some_and(|d| !d.charged.is_empty()) {
        let actor = game.lan_acting;
        let airborne = crate::game::tconstruct_projectile_airborne::airborne(game);
        if let (Some((velocity, spread)), Some(data)) = (game.player.as_ref().and_then(|player| crate::game::tconstruct_projectile_stats::projectile_stats_at_airborne(stack, player.position.y as f32, game.world.shape.max_y(), airborne)).or_else(|| crate::game::tconstruct_projectile_stats::projectile_stats(stack)), stack.data()) {
            let count = data.charged.len();
            let power = 3.0;
            // Source multishot repeats one ammo stack and marks its middle shot.
            // Keep legacy indexing for mixed, rocket, or unknown host payloads.
            let first = &data.charged[0];
            let primary = if matches!(&*first.item, "minecraft:arrow" | "minecraft:tipped_arrow" | "minecraft:spectral_arrow")
                && data.charged.iter().all(|ammo| ammo.same_item_same_components(first)) {
                count / 2
            } else { 0 };
            let shots = data.charged.iter().enumerate().filter(|(_, s)| &*s.item != "minecraft:firework_rocket").map(|(i, _)| {
                let angle = crate::game::tconstruct_projectile_stats::crossbow_angle(i, count);
                (angle, power, i == primary)
            }).collect();
            let glowing = crate::game::tconstruct_glowing_world::has_glowing(game, stack);
            let health = game.player.as_ref().map(|player| player.health.health());
            let maximum = if crate::game::tconstruct_projectile_stats::ambush_level(stack) > 0 {
                game.player.as_mut().and_then(|player| player.attribute("minecraft:max_health").ok()).map(|value| value as f32)
            } else { None };
            game.world.chunks.__horizon_mixins_tconstruct_material_projectile_context_mut().insert(actor, (velocity, spread, shots, false, crate::game::tconstruct_projectile_stats::projectile_bonus_at(stack, health, maximum), crate::game::tconstruct_projectile_stats::crystalbound_level(stack), glowing));
        }
        game.shoot_crossbow();
        game.world.chunks.__horizon_mixins_tconstruct_material_projectile_context_mut().remove(&actor);
    } else if game.creative || game.projectile_slot("minecraft:crossbow").is_some() {
        game.start_using_item();
    }
    Break(true)
}
