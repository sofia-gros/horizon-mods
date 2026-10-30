// Draw the item held by a casting block through Horizon's existing item
// renderer, with source fluid bounds. Cooling fade is separate work.
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
                        tconstruct_fluid_geometry(game, camera, out, *pos, &melter.tank, [0.08, 8.08, 0.08, 15.92, 15.92, 15.92], true);
                    }
                }
                continue;
            }
            if id == "tconstruct:faucet" {
                if &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name == "tconstruct:seared_faucet" {
                    if let Some(faucet) = raw.get("tconstruct:faucet").and_then(crate::game::tconstruct_source_melter_state::Faucet::from_nbt) {
                        if faucet.pending.fluid.is_some() && faucet.pending.amount > 0 {
                            let mut stream = faucet.pending.clone(); stream.amount = stream.capacity;
                            let state = game.pack.assets.registry.state(game.world.block(pos.0, pos.1, pos.2));
                            let facing = state.and_then(|state| state.properties.get("facing")).map(String::as_str).unwrap_or("north");
                            if facing == "down" {
                                tconstruct_fluid_geometry(game, camera, out, *pos, &stream, [6.0, 0.0, 6.0, 10.0, 16.0, 10.0], true);
                            } else {
                                let rotate = |bounds: [f32; 6]| -> [f32; 6] {
                                    let [x0,y0,z0,x1,y1,z1] = bounds;
                                    match facing {
                                        "south" => [16.0-x1,y0,16.0-z1,16.0-x0,y1,16.0-z0],
                                        "east" => [16.0-z1,y0,x0,16.0-z0,y1,x1],
                                        "west" => [z0,y0,16.0-x1,z1,y1,16.0-x0],
                                        _ => bounds,
                                    }
                                };
                                for bounds in [[6.0,6.0,0.0,10.0,9.0,6.0], [6.0,0.0,6.0,10.0,9.0,8.0]] {
                                    tconstruct_fluid_geometry(game, camera, out, *pos, &stream, rotate(bounds), true);
                                }
                            }
                        }
                    }
                }
                continue;
            }
            let (block, height, scale, flat) = match id.as_str() {
                "tconstruct:table" => ("tconstruct:seared_table", 15.5 / 16.0, 14.0 / 16.0, true),
                "tconstruct:basin" => ("tconstruct:seared_basin", 10.0 / 16.0, 12.0 / 16.0, false),
                _ => continue,
            };
            if &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name != block { continue; }
            let Some(casting) = raw.get(id.as_str()).and_then(crate::game::tconstruct_source_melter_state::Casting::from_nbt) else { continue; };
            if id == "tconstruct:table" {
                tconstruct_fluid_geometry(game, camera, out, *pos, &casting.tank, [1.0, 15.0, 1.0, 15.0, 15.9, 15.0], false);
            } else {
                tconstruct_fluid_geometry(game, camera, out, *pos, &casting.tank, [2.1, 4.0, 2.1, 13.9, 15.9, 13.9], true);
            }
            let Some(stack) = casting.output.as_ref().or(casting.input.as_ref()) else { continue; };
            let at = glam::DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
            if at.distance_squared(camera) >= 64.0 * 64.0 { continue; }
            let seed = horizon_core::position::BlockPos::new(pos.0, pos.1, pos.2).pack() as i32;
            let item = game.resolve_item(stack, DisplayContext::Fixed, None, seed);
            if item.is_empty() { continue; }
            let light = horizon_client::block_entity_render::vertex_light(game.world.light(pos.0, pos.1, pos.2));
            let rotation = if flat { glam::Mat4::from_rotation_y(180f32.to_radians()) * glam::Mat4::from_rotation_x(270f32.to_radians()) } else { glam::Mat4::IDENTITY };
            let pose = glam::Mat4::from_translation((at - camera).as_vec3() + glam::Vec3::new(0.5, height, 0.5))
                * rotation * glam::Mat4::from_scale(glam::Vec3::splat(scale));
            game.submit_item_world(&item, pose, light, out);
        }
    }

    // Fluid cuboid bounds and visible faces come from the source render provider.
fn tconstruct_fluid_geometry(game: &crate::game::Game<'_>, camera: glam::DVec3, out: &mut Vec<Vertex>, pos: (i32, i32, i32), tank: &crate::game::tconstruct_source_melter_state::Tank, bounds: [f32; 6], sides: bool) {
    let Some(fluid) = tank.fluid.as_deref().and_then(|id| id.strip_prefix("tconstruct:")) else { return; };
    let Some(sprite) = game.gui.atlas.sprites.get(&format!("gui/tconstruct_fluid_{fluid}")).copied() else { return; };
    let at = glam::DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
    if at.distance_squared(camera) >= 64.0 * 64.0 { return; }
    let origin = (at - camera).as_vec3();
    let [x0, y0, z0, x1, y1, z1] = bounds.map(|n| n / 16.0);
    let bottom = y0;
    let top = bottom + (y1 - bottom) * (tank.amount as f32 / tank.capacity as f32).clamp(0.0, 1.0);
    let light = horizon_client::block_entity_render::vertex_light(game.world.light(pos.0, pos.1, pos.2));
    let u0 = sprite.u0; let u1 = sprite.u0 + (sprite.u1 - sprite.u0) * (16.0 / sprite.w as f32);
    let v0 = sprite.v0; let v1 = sprite.v0 + (sprite.v1 - sprite.v0) * (16.0 / sprite.h as f32);
    let uv = [[u0,v0], [u1,v0], [u1,v1], [u0,v1]];
    let faces = [
        [[x0,top,z0], [x1,top,z0], [x1,top,z1], [x0,top,z1]],
        [[x0,top,z0], [x0,top,z1], [x0,bottom,z1], [x0,bottom,z0]],
        [[x1,top,z1], [x1,top,z0], [x1,bottom,z0], [x1,bottom,z1]],
        [[x1,top,z0], [x0,top,z0], [x0,bottom,z0], [x1,bottom,z0]],
        [[x0,top,z1], [x1,top,z1], [x1,bottom,z1], [x0,bottom,z1]],
    ];
    for face in faces.into_iter().take(if sides { 5 } else { 1 }) {
        for (p, uv) in face.into_iter().zip(uv) {
            out.push(Vertex { pos: (origin + glam::Vec3::from_array(p)).to_array(), uv, layer: 0,
                color: u32::MAX, light, normal: [0, 0, 0, 0], glint_uv: [0.0; 2] });
        }
    }
}

}
