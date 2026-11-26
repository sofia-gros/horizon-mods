fn prepare_fluid_render_bindings(
    parts: &PackParts,
) {
    crate::game::pluto_fluid_render_prepare::prepare(&parts.assets, &parts.render, &parts.tables);
}
