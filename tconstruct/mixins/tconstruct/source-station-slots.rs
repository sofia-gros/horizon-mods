fn station_visible_slots(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<Vec<usize>> {
    if crate::game::tconstruct_ui::kind(game) != Some(true) { return std::ops::ControlFlow::Continue(()); }
    let mut slots: Vec<usize> = (0..36).collect();
    slots.extend((0..game.container_size()).map(|index| 300 + index));
    slots.extend(500..505);
    std::ops::ControlFlow::Break(slots)
}
