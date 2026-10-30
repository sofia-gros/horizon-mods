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
    if !hand_changed {
        use horizon_world::nbt::Tag;
        let speed = match stack
            .data()
            .and_then(|d| d.other("minecraft:custom_data"))
            .and_then(|n| n.get("tic_draw_speed"))
        {
            Some(Tag::Float(bits)) => f32::from_bits(*bits),
            _ => 1.0,
        };
        let draw_ticks = (20.0 / speed).ceil().max(1.0);
        let adjusted = (ticks as f32 * 20.0 / draw_ticks).round() as u32;
        game.release_bow(adjusted);
    }
    Break(())
}
