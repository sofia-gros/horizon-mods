fn pluto_structure_natural_start(
    state:&StructureState,context:&mut impl StartContext,chunk:ChunkPos,wait:bool,
)->std::ops::ControlFlow<Option<Starts>> {
    pluto_structure_natural::build(state,context,chunk,wait)
}
