fn tconstruct_crossbow_use(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    if &*stack.item != "tconstruct:crossbow" {
        return Continue(());
    }
    if game.using.is_some() {
        return Break(true);
    }
    if stack.data().is_some_and(|d| !d.charged.is_empty()) {
        game.shoot_crossbow();
    } else if game.creative || game.projectile_slot("minecraft:crossbow").is_some() {
        game.using = Some((game.held_slot(), 0));
    }
    Break(true)
}
