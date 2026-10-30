// Draw the item held by a casting block through Horizon's existing item
// renderer. Fluid surfaces and the source cooling fade are separate work.
fn tconstruct_machine_geometry(game: &crate::game::Game<'_>, camera: glam::DVec3, out: &mut Vec<Vertex>) {
    use horizon_assets::item_model::DisplayContext;
    for (_, chunk) in &game.world.chunks {
        let Some(saved) = chunk.saved.as_deref() else { continue; };
        for (pos, raw) in &saved.block_entity_sources {
            let Some(horizon_world::nbt::Tag::String(id)) = raw.get("id") else { continue; };
            let id = id.to_string_lossy();
            if id == "tconstruct:melter" {
                if &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name == "tconstruct:seared_melter" {
                    if let Some(melter) = raw.get("tconstruct:melter").and_then(crate::game::tconstruct_source_melter_state::Melter::from_nbt) {
                        tconstruct_melter_fluid_geometry(game, camera, out, *pos, &melter);
                    }
                }
                continue;
            }
            let (block, height) = match id.as_str() {
                "tconstruct:table" => ("tconstruct:seared_table", 0.95),
                "tconstruct:basin" => ("tconstruct:seared_basin", 0.80),
                _ => continue,
            };
            if &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name != block { continue; }
            let Some(casting) = raw.get(id.as_str()).and_then(crate::game::tconstruct_source_melter_state::Casting::from_nbt) else { continue; };
            let Some(stack) = casting.output.as_ref().or(casting.input.as_ref()) else { continue; };
            let at = glam::DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
            if at.distance_squared(camera) >= 64.0 * 64.0 { continue; }
            let seed = horizon_core::position::BlockPos::new(pos.0, pos.1, pos.2).pack() as i32;
            let item = game.resolve_item(stack, DisplayContext::Fixed, None, seed);
            if item.is_empty() { continue; }
            let light = horizon_client::block_entity_render::vertex_light(game.world.light(pos.0, pos.1, pos.2));
            let pose = glam::Mat4::from_translation((at - camera).as_vec3() + glam::Vec3::new(0.5, height, 0.5))
                * glam::Mat4::from_rotation_x(90f32.to_radians()) * glam::Mat4::from_scale(glam::Vec3::splat(0.5));
            game.submit_item_world(&item, pose, light, out);
        }
    }
}

// The source half_tank model specifies the inner fluid bounds in sixteenths.
// Use its still texture and stored fill fraction for the five visible faces.
fn tconstruct_melter_fluid_geometry(game: &crate::game::Game<'_>, camera: glam::DVec3, out: &mut Vec<Vertex>, pos: (i32, i32, i32), melter: &crate::game::tconstruct_source_melter_state::Melter) {
    let Some(fluid) = melter.tank.fluid.as_deref().and_then(|id| id.strip_prefix("tconstruct:")) else { return; };
    let Some(sprite) = game.gui.atlas.sprites.get(&format!("gui/tconstruct_fluid_{fluid}")).copied() else { return; };
    let at = glam::DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
    if at.distance_squared(camera) >= 64.0 * 64.0 { return; }
    let origin = (at - camera).as_vec3();
    let (lo, hi, bottom) = (0.08_f32 / 16.0, 15.92_f32 / 16.0, 8.08_f32 / 16.0);
    let top = bottom + (15.92_f32 / 16.0 - bottom) * (melter.tank.amount as f32 / melter.tank.capacity as f32).clamp(0.0, 1.0);
    let light = horizon_client::block_entity_render::vertex_light(game.world.light(pos.0, pos.1, pos.2));
    let u0 = sprite.u0; let u1 = sprite.u0 + (sprite.u1 - sprite.u0) * (16.0 / sprite.w as f32);
    let v0 = sprite.v0; let v1 = sprite.v0 + (sprite.v1 - sprite.v0) * (16.0 / sprite.h as f32);
    let uv = [[u0,v0], [u1,v0], [u1,v1], [u0,v1]];
    let faces = [
        [[lo,top,lo], [hi,top,lo], [hi,top,hi], [lo,top,hi]],
        [[lo,top,lo], [lo,top,hi], [lo,bottom,hi], [lo,bottom,lo]],
        [[hi,top,hi], [hi,top,lo], [hi,bottom,lo], [hi,bottom,hi]],
        [[hi,top,lo], [lo,top,lo], [lo,bottom,lo], [hi,bottom,lo]],
        [[lo,top,hi], [hi,top,hi], [hi,bottom,hi], [lo,bottom,hi]],
    ];
    for face in faces {
        for (p, uv) in face.into_iter().zip(uv) {
            out.push(Vertex { pos: (origin + glam::Vec3::from_array(p)).to_array(), uv, layer: 0,
                color: u32::MAX, light, normal: [0, 0, 0, 0], glint_uv: [0.0; 2] });
        }
    }
}
