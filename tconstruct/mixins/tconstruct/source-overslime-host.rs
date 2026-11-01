// Match a synchronous damage calculation to the slot where it is committed.
use horizon_items::items::ItemStack;

pub(super) fn take(game: &mut super::Game<'_>, stack: &ItemStack) -> u32 {
    let actor = game.lan_acting;
    let pending = game.world.chunks.__horizon_mixins_tconstruct_overslime_pending_mut();
    let Some(entries) = pending.get_mut(&actor) else { return 0; };
    let Some(index) = entries.iter().position(|(item, _)| item == stack) else { return 0; };
    let (_, used) = entries.remove(index);
    if entries.is_empty() { pending.remove(&actor); }
    used
}

/// Shield is spent before the existing durability modifier chain.
pub(super) fn damage(game: &mut super::Game<'_>, stack: &ItemStack, amount: u32) -> (u32,u32) {
    if super::tconstruct_source_ranged_wear::is_broken(stack) { return (0,0); }
    let tanned = super::tconstruct_source_material_traits::levels(&game.world, stack)
        .and_then(|traits| traits.levels.get("tconstruct:tanned").copied()).unwrap_or(0);
    let amount = super::tconstruct_source_material_effects::tanned_wear(amount, tanned, None);
    let shield = super::tconstruct_source_overslime::amount(stack, "tic_overslime") as u32;
    let (remaining, residual) = super::tconstruct_source_overslime::absorb(shield, amount, None);
    (game.durability_loss(stack, residual), shield - remaining)
}
