fn tconstruct_golden_armor(value: bool, game: &crate::game::Game<'_>) -> bool {
    value || game.armor.iter().flatten().any(|stack| {
        stack.item.starts_with("tconstruct:") && crate::game::tconstruct_gold_guard_world::neutral(game, stack)
    })
}
