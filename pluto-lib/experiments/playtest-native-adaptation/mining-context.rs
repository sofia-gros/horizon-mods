// Private native borrow adapter, not a snapshot, registry, or mutation authority.
// The caller supplies the position and state from the current native operation.
// We do not re-read the block, validate ownership, project components, or clone Wire.
pub(super) struct BorrowedFacts<'a> {
    pub(super) facts: &'a horizon_items::blocks::StateFacts,
    pub(super) held: Option<&'a horizon_client::items::ItemStack>,
    pub(super) pos: [i32; 3],
    pub(super) max_y: i32,
}

pub(super) fn read<'a>(
    game: &'a crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
) -> BorrowedFacts<'a> {
    BorrowedFacts {
        facts: game.res.facts().get(state),
        held: game.held(),
        pos,
        max_y: game.world.shape.max_y(),
    }
}
