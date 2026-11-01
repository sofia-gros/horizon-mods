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
            let power = if data.charged.iter().any(|s| &*s.item == "minecraft:firework_rocket") { 1.6 } else { 3.15 };
            let shots = data.charged.iter().enumerate().filter(|(_, s)| &*s.item != "minecraft:firework_rocket").map(|(i, _)| {
                let angle = if count == 3 { [0.0, -10.0, 10.0][i] } else if count > 1 { -10.0 + 20.0 * i as f64 / (count - 1) as f64 } else { 0.0 };
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
