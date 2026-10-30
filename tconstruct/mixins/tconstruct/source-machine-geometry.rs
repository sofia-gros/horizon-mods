// Draw the item held by a casting block through Horizon's existing item
// renderer. Fluid surfaces and the source cooling fade are separate work.
fn tconstruct_machine_geometry(game: &crate::game::Game<'_>, camera: glam::DVec3, out: &mut Vec<Vertex>) {
    use horizon_assets::item_model::DisplayContext;
    for (_, chunk) in &game.world.chunks {
        let Some(saved) = chunk.saved.as_deref() else { continue; };
        for (pos, raw) in &saved.block_entity_sources {
            let Some(horizon_world::nbt::Tag::String(id)) = raw.get("id") else { continue; };
            let id = id.to_string_lossy();
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
