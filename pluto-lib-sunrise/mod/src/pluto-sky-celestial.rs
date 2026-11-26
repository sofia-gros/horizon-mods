// The frame's celestial bodies are built (the sun and moon quads, and the vanilla star frame):
// the mods' dynamic sky layers are asked for their quads, which join the bodies' own vertex list
// — untextured quads on the atlas's white texel, at the moon's light tier, in the bodies' own
// colour packing (alpha over blue over green over red) — and a layer that replaces the vanilla
// stars drops the vanilla star frame.
//
// The reach is a cancellable HEAD, not a RETURN: the game's own body builds the moon's sprite
// name with a bare format!, and the weaver cannot re-emit a body that names a macro without its
// full path (E_UNSUPPORTED_MACRO), so a return reach on this function can never weave. A HEAD
// reach leaves the body untouched: with no sink, or no layer for this frame, it continues and
// the game builds the bodies itself, exactly as it does unmodded. Only when a layer answers
// does the helper break, returning the bodies built the game's own way: the vertex list and
// the body list come from the host's own `celestial_bodies` (the same call the body makes),
// with the layer's quads joined and the star frame dropped when the layer says so.
fn pluto_sky_celestial(game: &crate::game::Game<'_>, sky: &sunrise_client::environment::Sky, rain: f32) -> core::ops::ControlFlow<(Vec<Vertex>, Vec<CelestialBody>, Option<(glam::Mat4, f32)>)> {
    let Some(sink) = crate::pluto_sky_access::sink() else {
        return core::ops::ControlFlow::Continue(());
    };
    let mut extra = Vec::new();
    let hide = (sink.frame)(sky.star_angle, sky.star_brightness, &mut extra);
    if extra.is_empty() && !hide {
        return core::ops::ControlFlow::Continue(());
    }
    let body_color =
        0x00FF_FFFF | (((1.0 - rain).clamp(0.0, 1.0) * 255.0).round() as u32) << 24;
    let phases = [
        "full_moon",
        "waning_gibbous",
        "third_quarter",
        "waning_crescent",
        "new_moon",
        "waxing_crescent",
        "first_quarter",
        "waxing_gibbous",
    ];
    let phase = phases[(game.view_day_time() / 24000).rem_euclid(8) as usize];
    let moon = std::format!("environment/celestial/moon/{phase}");
    let (mut out, bodies) = celestial_bodies(&game.gui.atlas.sprites, sky, &moon, body_color);
    // SkyRenderer.renderStars: the static star buffer on the same frame,
    // dropped when the layer replaces the vanilla stars.
    let stars = if hide {
        None
    } else {
        (sky.star_brightness > 0.0).then(|| (celestial_frame(sky.star_angle), sky.star_brightness))
    };
    if !extra.is_empty() {
        let uv = sunrise_client_render::gfx::lightning::white_uv(&game.gui.atlas.font.white());
        for quad in &extra {
            let channel = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u32;
            let color = channel(quad.rgba[3]) << 24 | channel(quad.rgba[2]) << 16 | channel(quad.rgba[1]) << 8 | channel(quad.rgba[0]);
            for corner in quad.corners {
                out.push(sunrise_client_render::mesher::Vertex { pos: corner, uv, layer: 2 | u32::from(sunrise_client_render::mesher::LOW) << 16, color, light: 0, normal: [0; 4], glint_uv: [0.0, 0.0] });
            }
        }
    }
    core::ops::ControlFlow::Break((out, bodies, stars))
}
