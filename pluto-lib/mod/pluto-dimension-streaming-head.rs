fn pluto_dimension_streaming_dispatch(
    res: &Resources, pack: &PackResources, options: &Options, settings: &Settings,
    world_root: &std::path::Path, dim: Dim, shape: horizon_client::chunk::LevelShape,
    seed: i64, enabled_features: &[String],
) -> std::ops::ControlFlow<Result<(Streaming, Arc<SaveDir>, Option<Arc<VanillaTerrain>>)>> {
    if dim == Dim::Overworld { return std::ops::ControlFlow::Continue(()); }
    std::ops::ControlFlow::Break(pluto_dimension_streaming::make_streaming(
        res, pack, options, settings, world_root, dim, shape, seed, enabled_features,
    ))
}
