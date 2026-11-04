fn tconstruct_bow_release(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    ticks: u32,
    hand_changed: bool,
) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    // release_used_item receives a clone after the host clears game.using.
    // Keep that clone's snapshot for this release, clear all live inventory copies.
    if matches!(&*stack.item, "tconstruct:longbow" | "tconstruct:crossbow") {
        crate::game::tconstruct_source_momentum_ranged_use::clear(game);
    }
    if &*stack.item != "tconstruct:longbow" {
        return Continue(());
    }
    if !hand_changed && !game.use_item_stopped(stack, ticks) { return Break(()); }
    if !hand_changed && !crate::game::tconstruct_source_ranged_wear::is_broken(stack) {
        let draw_ticks = crate::game::tconstruct_ranged_draw::draw_ticks(stack);
        let adjusted = crate::game::tconstruct_ranged_draw::vanilla_charge_ticks(ticks, draw_ticks);
        let actor = game.lan_acting;
        let mut dispatch_ticks = adjusted;
        let airborne = crate::game::tconstruct_projectile_airborne::airborne(game);
        if let Some((velocity, spread)) = game.player.as_ref().and_then(|player| crate::game::tconstruct_projectile_stats::projectile_stats_at_airborne(stack, player.position.y as f32, game.world.shape.max_y(), airborne)).or_else(|| crate::game::tconstruct_projectile_stats::projectile_stats(stack)) {
            let charge = crate::game::tconstruct_projectile_stats::bow_charge(ticks, draw_ticks);
            // Source compares charge * velocity before the host's 3x launch scale.
            if charge * velocity < 0.1 { return Break(()); }
            let power = charge * 3.0;
            dispatch_ticks = adjusted.max(3);
            let glowing = crate::game::tconstruct_glowing_world::has_glowing(game, stack);
            let health = game.player.as_ref().map(|player| player.health.health());
            let maximum = if crate::game::tconstruct_projectile_stats::ambush_level(stack) > 0 {
                game.player.as_mut().and_then(|player| player.attribute("minecraft:max_health").ok()).map(|value| value as f32)
            } else { None };
            game.world.chunks.__horizon_mixins_tconstruct_material_projectile_context_mut().insert(actor, (velocity, spread, vec![(0.0, power, true)], true, crate::game::tconstruct_projectile_stats::projectile_bonus_at(stack, health, maximum), crate::game::tconstruct_projectile_stats::crystalbound_level(stack), glowing, crate::game::tconstruct_ranged_draw::momentum_level(stack) > 0));
        }
        game.release_bow(stack, dispatch_ticks);
        game.world.chunks.__horizon_mixins_tconstruct_material_projectile_context_mut().remove(&actor);
    }
    Break(())
}
