fn tconstruct_storage_mesh_face(
    mesher: &Mesher<'_>, pad: &Padded, tints: &mut Tints<'_>, out: &mut SectionMesh,
    pos: [i32; 3], origin: [f32; 3], face: &BakedFace, ao: bool, state: &Info,
) -> std::ops::ControlFlow<()> {
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    let Some(base_layer) = mesher.render.sprite_layers.get("tconstruct:block/storage/fallback") else { return Continue(()); };
    if face.layer != *base_layer { return Continue(()); }
    let Some(chunk) = tints.region[1][1] else { return Continue(()); };
    let Some(saved) = chunk.saved.as_deref() else { return Continue(()); };
    let world = (tints.origin[0] * 16 + pos[0], tints.origin[2] + pos[1], tints.origin[1] * 16 + pos[2]);
    let material = saved.block_entities.iter().find_map(|raw| {
        if raw.get("id") != Some(&Tag::String("tconstruct:material_block".into())) { return None; }
        match (raw.get("x"), raw.get("y"), raw.get("z"), raw.get("Material")) {
            (Some(Tag::Int(x)), Some(Tag::Int(y)), Some(Tag::Int(z)), Some(Tag::String(id)))
                if (*x, *y, *z) == world => Some(id.to_string_lossy()),
            _ => None,
        }
    });
    let Some(material) = material else { return Continue(()); };
    let Some(data) = saved.root.get(&"tconstruct:mesh_render_info".into()).and_then(|tag| tag.get(&material)) else { return Continue(()); };
    let texture = match data.get("texture") { Some(Tag::String(id)) => Some(id.to_string_lossy()), _ => None };
    let fallbacks = match data.get("fallbacks") { Some(Tag::List(ids)) => ids.iter().filter_map(|id| match id { Tag::String(s) => Some(s.to_string_lossy()), _ => None }).collect(), _ => return Continue(()) };
    let color = match data.get("color") { Some(Tag::Int(v)) => *v as u32, _ => return Continue(()) };
    let luminosity = match data.get("luminosity") { Some(Tag::Int(v)) => *v as u8, _ => return Continue(()) };
    let info = tconstruct_storage_render_info::RenderInfo { texture, fallbacks, color, luminosity };
    let Ok((sprite, color, emission)) = tconstruct_storage_render_info::select_sprite_by("tconstruct:block/storage/fallback", &info, |id| mesher.render.sprite_layers.contains_key(id)) else { return Continue(()); };
    let Some(&layer) = mesher.render.sprite_layers.get(&sprite) else { return Continue(()); };
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
    let translucent = mesher.render.translucent_layers.get(layer as usize).copied().unwrap_or(false);
    let target = if translucent { &mut out.translucent } else { &mut out.solid };
    let emission = state.emission.max(q.light_emission).max(emission);
    for i in 0..4 {
        let g = gray[i] as u32;
        let rgb = [16, 8, 0].map(|shift| g * (color >> shift & 255) / 255);
        let p = q.positions[i];
        target.push(TerrainVertex::new(
            [p[0] + origin[0], p[1] + origin[1], p[2] + origin[2]], q.uv[i], layer as u16,
            emitting(pack_rgb(rgb), emission), pack_coords(lighting::with_emission(coords[i], emission)),
        ));
    }
    Break(())
}
