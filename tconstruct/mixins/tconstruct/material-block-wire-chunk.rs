fn material_block_wire_chunk(
    encoder: &crate::lan::chunks::ChunkEncoder,
    chunk: &crate::chunk::Chunk,
    world: &crate::world::World,
) -> std::ops::ControlFlow<Vec<u8>> {
    use horizon_world::nbt::{Compound, Tag};
    use std::ops::ControlFlow::{Break, Continue};
    let Some((block, kind)) = *world.chunks.__horizon_mixins_tconstruct_material_wire_ids() else { return Continue(()); };
    let Some(saved) = chunk.saved.as_deref() else { return Continue(()); };
    let mut materials = HashMap::<(i32, i32, i32), Tag>::new();
    for raw in &saved.block_entities {
        if raw.get("id") != Some(&Tag::String("tconstruct:material_block".into())) { continue; }
        let pos = match (raw.get("x"), raw.get("y"), raw.get("z")) {
            (Some(Tag::Int(x)), Some(Tag::Int(y)), Some(Tag::Int(z))) => (*x, *y, *z),
            _ => continue,
        };
        if (pos.0 >> 4, pos.2 >> 4) != (chunk.x, chunk.z) { continue; }
        let state = chunk.block(pos.0 & 15, pos.1, pos.2 & 15).0 as usize;
        if encoder.block_ids.get(state).copied() != Some(block) { continue; }
        let Some(material) = raw.get("Material") else { continue; };
        let mut tag = Compound::new();
        tag.insert("Material".into(), material.clone());
        materials.insert(pos, Tag::Compound(tag));
    }
    if materials.is_empty() { return Continue(()); }
    let (wires, mut entities) = encoder.blocks(chunk, &world.block_entities);
    for entity in &mut entities {
        if entity.kind != kind { continue; }
        let x = chunk.x * 16 + i32::from(entity.packed_xz as u8 >> 4);
        let z = chunk.z * 16 + i32::from(entity.packed_xz as u8 & 15);
        if let Some(tag) = materials.get(&(x, i32::from(entity.y), z)) { entity.tag = tag.clone(); }
    }
    // The remainder is the pinned Horizon ChunkEncoder::body serialization,
    // retaining its section, light and block-entity order.
    let heightmaps = encoder.heightmaps_of(chunk);
    let lights = encoder.light_sections(chunk, encoder.around(chunk, world));
    let buffer: usize = wires.iter().map(|w| w.len()).sum();
    let mut tags = Writer::new();
    List::<BlockEntityInfo>::write(&entities, &mut tags);
    let size = 64 + heightmaps.iter().map(|(_, h)| 10 + 8 * h.len()).sum::<usize>()
        + buffer + tags.data.len() + lights.len() * 2 * (2048 + 3);
    let mut w = Writer { data: Vec::with_capacity(size) };
    w.varint(horizon_protocol::ids::play::clientbound::LEVEL_CHUNK_WITH_LIGHT);
    w.i32(chunk.x);
    w.i32(chunk.z);
    Heightmaps::write(&*heightmaps, &mut w);
    w.varint(buffer as i32);
    for wire in &wires { w.bytes(wire); }
    w.bytes(&tags.data);
    write_light(&lights, encoder.sky, &mut w);
    Break(w.data)
}
