fn tconstruct_remote_bow_start(
    game: &mut crate::game::Game<'_>,
    held: &horizon_client::items::ItemStack,
) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if &*held.item != "tconstruct:longbow" {
        return Continue(());
    }
    if game.using.is_none() && (game.creative || game.projectile_slot("minecraft:bow").is_some()) {
        game.using = Some((game.held_slot(), 0));
    }
    Break(())
}
