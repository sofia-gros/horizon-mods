fn prepare_fluid_render_bindings(
    assets: &Arc<Assets>,
    render: &Arc<RenderAssets>,
    tables: &Arc<MeshTables>,
) {
    crate::game::pluto_fluid_render_prepare::prepare(assets, render, tables);
}
