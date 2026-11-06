// Draw the item held by a casting block through Horizon's existing item
// renderer, with source fluid bounds. Cooling fade is separate work.
fn tconstruct_machine_geometry(game: &crate::game::Game<'_>, camera: glam::DVec3, out: &mut Vec<Vertex>) {
    use horizon_assets::item_model::DisplayContext;
    for (_, chunk) in &game.world.chunks {
        let Some(saved) = chunk.saved.as_deref() else { continue; };
        for (pos, raw) in &saved.block_entity_sources {
            let Some(horizon_world::nbt::Tag::String(id)) = raw.get("id") else { continue; };
            let id = id.to_string_lossy();
            if id == "tconstruct:fluid_cannon" {
                if &*game.res.facts().get(game.world.block(pos.0,pos.1,pos.2)).name == "tconstruct:seared_fluid_cannon" {
                    if let Some(state) = raw.get(id.as_str()).and_then(crate::game::tconstruct_source_fluid_cannon::Cannon::from_nbt) {
                        tconstruct_fluid_geometry(game, camera, out, *pos, &state.tank, [0.08,8.08,0.08,15.92,15.92,15.92], true);
                    }
                }
                continue;
            }
            if id == "tconstruct:casting_tank" {
                if &*game.res.facts().get(game.world.block(pos.0,pos.1,pos.2)).name == "tconstruct:seared_casting_tank" {
                    if let Some(state) = raw.get(id.as_str()).and_then(crate::game::tconstruct_source_casting_tank::CastingTank::from_nbt) {
                        tconstruct_fluid_geometry(game, camera, out, *pos, &state.tank, [0.08,0.08,0.08,15.92,10.92,15.92], true);
                        if let Some(stack) = state.slots[1].as_ref().or(state.slots[0].as_ref()) {
                            let at = glam::DVec3::new(f64::from(pos.0),f64::from(pos.1),f64::from(pos.2));
                            if at.distance_squared(camera) < 64.0 * 64.0 {
                                let seed = horizon_core::position::BlockPos::new(pos.0,pos.1,pos.2).pack() as i32;
                                let item = game.resolve_item(stack, DisplayContext::Fixed, None, seed);
                                if !item.is_empty() {
                                    let light = horizon_client::block_entity_render::vertex_light(game.world.light(pos.0,pos.1,pos.2));
                                    let pose = glam::Mat4::from_translation((at-camera).as_vec3() + glam::Vec3::new(0.5,15.5/16.0,0.5))
                                        * glam::Mat4::from_rotation_x(270f32.to_radians()) * glam::Mat4::from_scale(glam::Vec3::splat(0.5));
                                    game.submit_item_world(&item, pose, light, out);
                                }
                            }
                        }
                    }
                }
                continue;
            }
            if matches!(id.as_str(), "tconstruct:fuel_tank" | "tconstruct:ingot_tank") {
                let block = &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name;
                let expected = match id.as_str() {
                    "tconstruct:fuel_tank" => matches!(block, "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge" | "tconstruct:scorched_fuel_tank" | "tconstruct:scorched_fuel_gauge"),
                    _ => matches!(block, "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge" | "tconstruct:scorched_ingot_tank" | "tconstruct:scorched_ingot_gauge"),
                };
                if expected {
                    let capacity = if id == "tconstruct:fuel_tank" { 4000 } else { 4320 };
                    if let Some(tank) = raw.get(id.as_str()).and_then(|tag| crate::game::tconstruct_source_melter_state::Tank::from_nbt(tag, capacity)) {
                        tconstruct_fluid_geometry(game, camera, out, *pos, &tank, [0.08, 0.08, 0.08, 15.92, 15.92, 15.92], true);
                    }
                }
                continue;
            }
            if id == "tconstruct:melter" {
                if &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name == "tconstruct:seared_melter" {
                    if let Some(melter) = raw.get("tconstruct:melter").and_then(crate::game::tconstruct_source_melter_state::Melter::from_nbt) {
                        tconstruct_fluid_geometry(game, camera, out, *pos, &melter.tank, [0.08, 8.08, 0.08, 15.92, 15.92, 15.92], true);
                    }
                }
                continue;
            }
            if id == "tconstruct:faucet" {
                if matches!(&*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name, "tconstruct:seared_faucet" | "tconstruct:scorched_faucet") {
                    if let Some(faucet) = raw.get("tconstruct:faucet").and_then(crate::game::tconstruct_source_melter_state::Faucet::from_nbt) {
                        if faucet.pending.fluid.is_some() && faucet.pending.amount > 0 {
                            let mut stream = faucet.pending.clone(); stream.amount = stream.capacity;
                            let state = game.pack.assets.registry.state(game.world.block(pos.0, pos.1, pos.2));
                            let facing = state.and_then(|state| state.properties.get("facing")).map(String::as_str).unwrap_or("north");
                            if facing == "down" {
                                tconstruct_fluid_geometry(game, camera, out, *pos, &stream, [6.0, 0.0, 6.0, 10.0, 16.0, 10.0], true);
                            } else {
                                for bounds in crate::game::tconstruct_source_melter_render::faucet_stream_bounds(facing) {
                                    tconstruct_fluid_geometry(game, camera, out, *pos, &stream, bounds, true);
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
            let actual = &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name;
            if actual != block && !((id == "tconstruct:table" && actual == "tconstruct:scorched_table") || (id == "tconstruct:basin" && actual == "tconstruct:scorched_basin")) { continue; }
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
            let context = if flat { DisplayContext::Fixed } else { DisplayContext::None };
            let item = game.resolve_item(stack, context, None, seed);
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
    let Some(fluid) = tank.fluid.as_deref() else { return; };
    let name = crate::game::tconstruct_source_melter_render::sprite(fluid);
    let Some(sprite) = horizon_client::pluto_render::sprite(&game.gui.atlas, &name).or_else(|| horizon_client::pluto_render::sprite(&game.gui.atlas, "block/tconstruct_fluid_fallback")) else { return; };
    let at = glam::DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
    if at.distance_squared(camera) >= 64.0 * 64.0 { return; }
    let origin = (at - camera).as_vec3();
    let [x0, y0, z0, x1, y1, z1] = bounds.map(|n| n / 16.0);
    let bottom = y0;
    let top = bottom + (y1 - bottom) * (tank.amount as f32 / tank.capacity as f32).clamp(0.0, 1.0);
    let light = crate::game::tconstruct_source_melter_render::fluid_light(game.world.light(pos.0, pos.1, pos.2), fluid);
    let color = crate::game::tconstruct_source_melter_render::color_with_complete(fluid, tank.tag.as_ref(), game.remote.is_none());
    let faces = if sides { horizon_client::pluto_render::TOP_AND_SIDES } else { horizon_client::pluto_render::TOP };
    let _ = horizon_client::pluto_render::cuboid(out, sprite, origin.to_array(),
        [x0, bottom, z0, x1, top, z1], faces, [16.0, 16.0], color, light);
}

}
