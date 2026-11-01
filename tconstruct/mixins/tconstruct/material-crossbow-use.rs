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
        if let (Some((velocity, spread)), Some(data)) = (crate::game::tconstruct_projectile_stats::projectile_stats(stack), stack.data()) {
            let count = data.charged.len();
            let power = 3.0;
            let shots = data.charged.iter().enumerate().filter(|(_, s)| &*s.item != "minecraft:firework_rocket").map(|(i, _)| {
                let angle = (i as f64 - (count.saturating_sub(1) as f64 / 2.0)) * 10.0;
                (angle, power)
            }).collect();
            game.world.chunks.__horizon_mixins_tconstruct_material_projectile_context_mut().insert(actor, (velocity, spread, shots, false));
        }
        game.shoot_crossbow();
        game.world.chunks.__horizon_mixins_tconstruct_material_projectile_context_mut().remove(&actor);
    } else if game.creative || game.projectile_slot("minecraft:crossbow").is_some() {
        game.using = Some((game.held_slot(), 0));
    }
    Break(true)
}
