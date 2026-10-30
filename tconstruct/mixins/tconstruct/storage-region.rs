fn tconstruct_storage_region(world: &World, x: i32, z: i32) -> std::ops::ControlFlow<Option<Box<Region>>> {
    use horizon_world::nbt::{Compound, Tag};
    use std::ops::ControlFlow::{Break, Continue};
    let Some(Ok((records, _))) = world.chunks.__horizon_mixins_tconstruct_render_assets().get() else { return Continue(()); };
    let Some(center) = world.chunks.get(&(x, z)) else { return Continue(()); };
    let Some(saved) = center.saved.as_deref() else { return Continue(()); };
    let mut values = Compound::new();
    for raw in &saved.block_entities {
        if raw.get("id") != Some(&Tag::String("tconstruct:material_block".into())) { continue; }
        let Some(Tag::String(material)) = raw.get("Material") else { continue; };
        let material = material.to_string_lossy();
        if material == "tconstruct:unknown" { continue; }
        let Ok(Some(info)) = tconstruct_storage_render_info::render_info(records, &material) else { continue; };
        let mut data = Compound::new();
        if let Some(texture) = info.texture { data.insert("texture".into(), Tag::String(texture.as_str().into())); }
        data.insert("fallbacks".into(), Tag::List(info.fallbacks.into_iter().map(|id| Tag::String(id.as_str().into())).collect()));
        data.insert("color".into(), Tag::Int(info.color as i32));
        data.insert("luminosity".into(), Tag::Int(i32::from(info.luminosity)));
        values.insert(material.as_str().into(), Tag::Compound(data));
    }
    if values.is_empty() { return Continue(()); }
    let mut region: Region = Default::default();
    for dz in 0..3 {
        for dx in 0..3 {
            let Some(chunk) = world.chunks.get(&(x + dx as i32 - 1, z + dz as i32 - 1)) else { return Break(None); };
            region[dz][dx] = Some(chunk.clone());
        }
    }
    let chunk = Arc::make_mut(region[1][1].as_mut().expect("center chunk"));
    let saved = Arc::make_mut(chunk.saved.as_mut().expect("saved block entities"));
    saved.root.insert("tconstruct:mesh_render_info".into(), Tag::Compound(values));
    Break(Some(Box::new(region)))
}
