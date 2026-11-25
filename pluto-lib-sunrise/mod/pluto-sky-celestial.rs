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
// does the helper break, returning the bodies built the game's own way (the transcription
// below mirrors celestial_geometry in game/frame_geometry.rs line for line, with the layer's
// quads joined and the star frame dropped when the layer says so).
fn pluto_sky_celestial(game: &crate::game::Game<'_>, sky: &sunrise_client::environment::Sky, rain: f32) -> core::ops::ControlFlow<(Vec<Vertex>, Option<(glam::Mat4, f32)>)> {
    let Some(sink) = crate::entry::pluto_sky_access::sink() else {
        return core::ops::ControlFlow::Continue(());
    };
    let mut extra = Vec::new();
    let hide = (sink.frame)(sky.star_angle, sky.star_brightness, &mut extra);
    if extra.is_empty() && !hide {
        return core::ops::ControlFlow::Continue(());
    }
    let body_color =
        0x00FF_FFFF | (((1.0 - rain).clamp(0.0, 1.0) * 255.0).round() as u32) << 24;
    let mut out = Vec::new();
    let atlas = &game.gui.atlas;
    let frame = |angle: f32| {
        glam::Mat4::from_rotation_y((-90.0f32).to_radians())
            * glam::Mat4::from_rotation_x(angle.to_radians())
    };
    let mut body = |name: &str, angle: f32, size: f32, moon: bool| {
        let Some(r) = atlas.sprites.get(name).copied() else {
            return;
        };
        // How brightly HDR output lets the body shine (mesher::hdr_tier):
        // the moon at the low tier, not the sun's.
        let tier = if moon {
            sunrise_client::mesher::LOW
        } else {
            sunrise_client::mesher::VERY_BRIGHT
        };
        let m = frame(angle);
        let corners = [(-size, -size), (size, -size), (size, size), (-size, size)];
        // buildCelestialQuad (sun) vs buildMoonPhases: the moon's UVs run
        // the other way round.
        let uv = if moon {
            [[r.u1, r.v1], [r.u0, r.v1], [r.u0, r.v0], [r.u1, r.v0]]
        } else {
            [[r.u0, r.v0], [r.u1, r.v0], [r.u1, r.v1], [r.u0, r.v1]]
        };
        for ((x, z), uv) in corners.iter().zip(uv) {
            let p = m.transform_point3(glam::Vec3::new(*x, 100.0, *z));
            out.push(sunrise_client::mesher::Vertex {
                normal: [0; 4],
                pos: p.to_array(),
                uv,
                layer: 2 | u32::from(tier) << 16,
                color: body_color,
                light: 0,
                glint_uv: [0.0, 0.0],
            });
        }
    };
    body("environment/celestial/sun", sky.sun_angle, 30.0, false);
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
    body(
        &std::format!("environment/celestial/moon/{phase}"),
        sky.moon_angle,
        20.0,
        true,
    );
    // SkyRenderer.renderStars: the static star buffer on the same frame,
    // dropped when the layer replaces the vanilla stars.
    let stars = if hide {
        None
    } else {
        (sky.star_brightness > 0.0).then(|| (frame(sky.star_angle), sky.star_brightness))
    };
    if !extra.is_empty() {
        let uv = sunrise_client::gfx::lightning::white_uv(&game.gui.atlas.font.white());
        for quad in &extra {
            let channel = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u32;
            let color = channel(quad.rgba[3]) << 24 | channel(quad.rgba[2]) << 16 | channel(quad.rgba[1]) << 8 | channel(quad.rgba[0]);
            for corner in quad.corners {
                out.push(sunrise_client::mesher::Vertex { pos: corner, uv, layer: 2 | u32::from(sunrise_client::mesher::LOW) << 16, color, light: 0, normal: [0; 4], glint_uv: [0.0, 0.0] });
            }
        }
    }
    core::ops::ControlFlow::Break((out, stars))
}
