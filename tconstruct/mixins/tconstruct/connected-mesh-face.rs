fn tconstruct_connected_mesh_face(
    mesher: &Mesher<'_>, pad: &Padded, tints: &mut Tints<'_>, out: &mut SectionMesh,
    pos: [i32; 3], origin: [f32; 3], face: &BakedFace, ao: bool, state: &Info,
) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let registry = mesher.assets.registry;
    let Some(current) = registry.state(BlockStateId(pad.blocks[pi(pos[0], pos[1], pos[2])])) else { return Continue(()); };
    let Some((pane, color)) = tconstruct_connected_texture::glass_kind(&current.name) else { return Continue(()); };
    let sprite = face.quad.material.sprite.as_str();
    let connected = matches!(sprite, "tconstruct:block/clear_glass" | "tconstruct:block/clear_stained_glass" | "tconstruct:block/smeltery/seared_glass" | "tconstruct:block/smeltery/soul_glass" | "tconstruct:block/foundry/glass" | "tconstruct:block/foundry/soul_glass");
    if !connected && color == 0xffffff { return Continue(()); }
    let mut layer = face.layer;
    if connected {
        let Some(directions) = tconstruct_connected_texture::baked_uv_directions(face.quad.positions, face.quad.uv) else { return Continue(()); };
        let has_connections = |state: &horizon_world::chunk::BlockState| {
            ["north", "south", "west", "east"].iter().any(|key| state.properties.get(*key).is_some_and(|value| value == "true"))
        };
        let adjacent = directions.map(|direction| {
            let [dx, dy, dz] = direction.normal();
            let other = pad.blocks[pi(pos[0] + dx, pos[1] + dy, pos[2] + dz)];
            registry.state(BlockStateId(other)).is_some_and(|other| current.name == other.name && (!pane || has_connections(current) == has_connections(other)))
        });
        let suffix = tconstruct_connected_texture::cornerless_suffix(adjacent[0], adjacent[1], adjacent[2], adjacent[3]);
        if !suffix.is_empty() {
            let Some(&found) = mesher.render.sprite_layers.get(&format!("{sprite}/{suffix}")) else { return Continue(()); };
            layer = found;
        }
    }
    let q = &face.quad;
    let directional = lighting::cardinal(mesher.nether_shading, q.shade_direction_override.unwrap_or(q.facing));
    let (gray, coords) = if ao {
        let (brightness, coords) = lighting::ambient_occlusion(pad, &mesher.tables.info, state, pos, &q.positions, q.facing);
        (brightness.map(|b| lighting::scale(lighting::gray(b), directional)), coords)
    } else {
        let coords = match q.cull {
            Some(d) => lighting::coords(pad, state, lighting::step(pos, d)),
            None => lighting::flat(pad, state, pos, &q.positions, q.facing),
        };
        ([lighting::gray(directional); 4], [coords; 4])
    };
    let target = if face.translucent { &mut out.translucent } else { &mut out.solid };
    for i in 0..4 {
        let g = gray[i] as u32;
        let rgb = [16, 8, 0].map(|shift| g * (color >> shift & 255) / 255);
        let p = q.positions[i];
        target.push(TerrainVertex::new(
            [p[0] + origin[0], p[1] + origin[1], p[2] + origin[2]], q.uv[i], layer as u16,
            emitting(pack_rgb(rgb), state.emission.max(q.light_emission)),
            pack_coords(lighting::with_emission(coords[i], q.light_emission)),
        ));
    }
    Break(())
}
