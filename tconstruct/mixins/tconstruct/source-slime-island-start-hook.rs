fn tconstruct_island_start(
    registry: &StructureRegistry, resources: &StructureResources,
    context: &mut impl StartContext, structure: usize, chunk: ChunkPos,
) -> std::ops::ControlFlow<Option<StructureStart>> {
    if !matches!(registry.structures.get(structure).map(|s| &s.kind),
        Some(StructureKind::Unsupported(id)) if id == "tconstruct:island") {
        return std::ops::ControlFlow::Continue(());
    }
    std::ops::ControlFlow::Break(tconstruct_slime_island_start::generate(registry, resources, context, structure, chunk))
}
