fn pluto_structure_generation_start(
    decoration:&Decoration,data:&Arc<GeneratorData>,seed:i64,generation:WorldGenerationContext,
    worker:&mut WorkerState,region:&mut [ProtoChunk;9],
    level_random:&mut dyn super::structure::legacy::PieceRandom,
    structures:Option<&super::generator_structures::StructureState>,
)->std::ops::ControlFlow<Result<DecorateStats,GeneratorError>> {
    if !pluto_structure_generation::has_registered(region) {return std::ops::ControlFlow::Continue(())}
    std::ops::ControlFlow::Break(decoration.pluto_decorate(data,seed,generation,worker,region,level_random,structures))
}
