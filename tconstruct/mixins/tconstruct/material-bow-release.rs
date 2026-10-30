fn tconstruct_bow_release(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, ticks: u32, hand_changed: bool) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if &*stack.item != "tconstruct:longbow" { return Continue(()); }
    if !hand_changed { game.release_bow(ticks); }
    Break(())
}
