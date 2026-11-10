fn tconstruct_shield_raise(
    game: &mut crate::game::Game<'_>,
    slot: usize,
) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(stack) = game.stack_in(slot) else {
        return Continue(());
    };
    if !crate::game::tconstruct_shield::stock(&stack.item) {
        return Continue(());
    }
    let valid = if game.remote.is_some() {
        crate::game::tconstruct_shield_world::valid_stack(stack)
    } else {
        crate::game::tconstruct_shield_world::stats(game, stack).is_some()
    };
    if !valid {
        return Break(false);
    }
    // Native hand selection, cooldown and 72000-tick use lifecycle remain intact.
    Continue(())
}
