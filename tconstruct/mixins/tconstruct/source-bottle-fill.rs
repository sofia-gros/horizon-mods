use super::*;

/// Reuse the host's ItemUtils.createFilledResult path for both bottle forms.
pub(in crate::game) fn fill(game: &mut Game<'_>, filled: ItemStack) {
    game.fill_held(filled);
}
