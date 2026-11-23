// The frame's celestial bodies are built (the sun and moon quads, and the vanilla star frame):
// the mods' dynamic sky layers are asked for their quads, which join the bodies' own vertex list
// — untextured quads on the atlas's white texel, at the moon's light tier, in the bodies' own
// colour packing (alpha over blue over green over red) — and a layer that replaces the vanilla
// stars drops the vanilla star frame. With no sink or no layer, the bodies are exactly the
// game's own.
fn pluto_sky_celestial(value:(Vec<horizon_client::mesher::Vertex>,Option<(glam::Mat4,f32)>),game:&crate::game::Game<'_>,sky:&horizon_client::environment::Sky,rain:f32)->(Vec<horizon_client::mesher::Vertex>,Option<(glam::Mat4,f32)>) {
    let (mut quads, stars) = value;
    let Some(sink)=crate::entry::pluto_sky_access::sink() else {return (quads, stars)};
    let mut extra=Vec::new();
    let hide=(sink.frame)(sky.star_angle, sky.star_brightness, &mut extra);
    if !extra.is_empty() {
        let uv=horizon_client::gfx::lightning::white_uv(&game.gui.atlas.font.white());
        for quad in &extra {
            let channel=|c:f32| (c.clamp(0.0, 1.0)*255.0).round() as u32;
            let color=channel(quad.rgba[3])<<24 | channel(quad.rgba[2])<<16 | channel(quad.rgba[1])<<8 | channel(quad.rgba[0]);
            for corner in quad.corners {
                quads.push(horizon_client::mesher::Vertex {pos:corner, uv, layer:2 | u32::from(horizon_client::mesher::LOW) << 16, color, light:0, normal:[0; 4], glint_uv:[0.0, 0.0]});
            }
        }
    }
    (quads, if hide {None} else {stars})
}
