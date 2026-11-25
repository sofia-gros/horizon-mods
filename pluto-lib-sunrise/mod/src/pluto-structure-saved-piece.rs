fn pluto_structure_saved_piece(
    state:&StructureState,piece:&crate::nbt::Tag,
)->std::ops::ControlFlow<Option<Piece>> {
    pluto_structure_context::saved_piece(state,piece)
}
