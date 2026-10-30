fn tconstruct_bow_tick(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some((slot, ticks)) = game.using else { return Continue(()); };
    let Some(stack) = game.stack_in(slot).cloned() else { return Continue(()); };
    if &*stack.item != "tconstruct:longbow" || !game.right || (slot != game.selected && slot != crate::game::shield::OFFHAND_SLOT) || game.screen != crate::game::Screen::Playing { return Continue(()); }
    game.trigger_using_item(&stack);
    game.using = Some((slot, ticks + 1));
    Break(())
}
