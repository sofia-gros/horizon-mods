fn tconstruct_bow_tick(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some((slot, ticks)) = game.using else {
        return Continue(());
    };
    let Some(stack) = game.stack_in(slot).cloned() else {
        return Continue(());
    };
    if !matches!(&*stack.item, "tconstruct:longbow" | "tconstruct:crossbow")
        || !game.right
        || (slot != game.selected && slot != crate::game::shield::OFFHAND_SLOT)
        || game.screen != crate::game::Screen::Playing
    {
        return Continue(());
    }
    game.trigger_using_item(&stack);
    // A concurrent edible modifier may have changed or broken the launcher.
    let Some(current) = game.stack_in(slot).cloned() else { return Break(()); };
    if &*current.item == "tconstruct:crossbow" {
        if crate::game::tconstruct_source_melee_shape::intact(&current).is_some() {
            game.tick_crossbow(slot, ticks, &current);
        } else {
            game.using = Some((slot, ticks + 1));
        }
    } else {
        game.using = Some((slot, ticks + 1));
    }
    Break(())
}
