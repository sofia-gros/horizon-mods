// Reuse the engine's inventory preview projection from its declaring module.
pub(in crate::game) fn player_preview_quads(
    verts: Vec<horizon_client::mesher::Vertex>,
    gui: glam::Mat4,
    fire: Option<(f32, f32)>,
    sprites: &std::collections::HashMap<String, horizon_client::hud::Rect>,
) -> Vec<(f32, [[f32; 3]; 4], [[f32; 2]; 4], u32)> {
    super::player_preview_quads(verts, gui, fire, sprites)
}
