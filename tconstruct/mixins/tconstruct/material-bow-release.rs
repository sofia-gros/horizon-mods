fn tconstruct_bow_release(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    ticks: u32,
    hand_changed: bool,
) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if &*stack.item != "tconstruct:longbow" {
        return Continue(());
    }
    if !hand_changed && !crate::game::tconstruct_source_ranged_wear::is_broken(stack) {
        let draw_ticks = crate::game::tconstruct_ranged_draw::draw_ticks(stack);
        let adjusted = crate::game::tconstruct_ranged_draw::vanilla_charge_ticks(ticks, draw_ticks);
        let actor = game.lan_acting;
        let mut dispatch_ticks = adjusted;
        if let Some((velocity, spread)) = crate::game::tconstruct_projectile_stats::projectile_stats(stack) {
            let power = crate::game::tconstruct_projectile_stats::bow_charge(ticks, draw_ticks) * 3.0;
            if power * velocity < 0.3 { return Break(()); }
            dispatch_ticks = adjusted.max(3);
            game.world.chunks.__horizon_mixins_tconstruct_material_projectile_context_mut().insert(actor, (velocity, spread, vec![(0.0, power)], true, crate::game::tconstruct_projectile_stats::projectile_bonus(stack)));
        }
        game.release_bow(dispatch_ticks);
        game.world.chunks.__horizon_mixins_tconstruct_material_projectile_context_mut().remove(&actor);
    }
    Break(())
}
