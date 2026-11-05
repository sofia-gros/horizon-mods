// Re-read both hands at the native status update, including same-item material
// edits and durability changes which do not change the selected item ID.
fn tconstruct_crystalstrike_status(game: &mut crate::game::Game<'_>) {
    crate::game::tconstruct_crystalstrike_world::refresh(game);
}
