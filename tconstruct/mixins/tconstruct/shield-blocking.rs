fn tconstruct_shield_blocking(
    game: &crate::game::Game<'_>,
    from: Option<[f64; 3]>,
    damage_type: &str,
    pierce: bool,
) -> std::ops::ControlFlow<Option<Blocking>> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(stack) = game.using.and_then(|(slot, _)| game.stack_in(slot)) else {
        return Continue(());
    };
    if !crate::game::tconstruct_shield::stock(&stack.item) {
        return Continue(());
    }
    Break(crate::game::tconstruct_shield_world::blocking(
        game,
        from,
        damage_type,
        pierce,
    ))
}
