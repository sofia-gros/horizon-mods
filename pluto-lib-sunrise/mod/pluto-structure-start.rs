fn pluto_structure_start(
    state:&StructureState,context:&mut impl StartContext,structure:&str,chunk:ChunkPos,
)->std::ops::ControlFlow<Option<StructureStart>> {
    pluto_structure_context::explicit_start(state,context,structure,chunk)
}
