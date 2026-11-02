fn tconstruct_chest_slots(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<Vec<usize>> {
    let Some((count, scroll)) = crate::game::tconstruct_chest_ui::page(game) else { return std::ops::ControlFlow::Continue(()); };
    let first = scroll * crate::game::tconstruct_chest_rules::CHEST_COLUMNS;
    let mut slots: Vec<usize> = (0..36).collect();
    slots.extend((first..count.min(first + 32)).map(|index| 300 + index));
    std::ops::ControlFlow::Break(slots)
}
