// A use has one active slot (including offhand). Remove abandoned snapshots
// before a new use, rather than carrying an old actor's speed with the item.
pub(super) fn clear(game: &mut crate::game::Game<'_>) {
    for stack in game.inventory.iter_mut().chain(std::iter::once(&mut game.offhand)).flatten() {
        if matches!(&*stack.item, "tconstruct:longbow" | "tconstruct:crossbow") {
            super::tconstruct_ranged_draw::clear_snapshot(stack);
        }
    }
}

pub(super) fn start(game: &mut crate::game::Game<'_>) -> bool {
    if game.using.is_some() { return false; }
    clear(game);
    let started = game.held().cloned();
    let effect_level = game.potions.effects.amplifier("tconstruct:momentum_ranged")
        .map_or(0, |amplifier| amplifier.saturating_add(1));
    if !game.start_using_item() { return false; }
    let Some((slot, _)) = game.using else { return false; };
    if let Some(stack) = game.stack_in_mut(slot).as_mut() {
        if matches!(&*stack.item, "tconstruct:longbow" | "tconstruct:crossbow")
            && started.as_ref().is_some_and(|before| before.same_item_same_components(stack)) {
            super::tconstruct_ranged_draw::snapshot(stack, effect_level);
        }
    }
    true
}
