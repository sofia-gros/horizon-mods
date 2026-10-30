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
        game.release_bow(adjusted);
    }
    Break(())
}
