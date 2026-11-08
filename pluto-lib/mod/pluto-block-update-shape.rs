fn pluto_block_update_shape(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, direction: Direction, neighbour_pos: BlockPos, neighbour: BlockStateId, new: &mut Option<BlockStateId>) {
    super::pluto_block_level::update_shape(level, pos, state, direction, neighbour_pos, neighbour, new);
}
