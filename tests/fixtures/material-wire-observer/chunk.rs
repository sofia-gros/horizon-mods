fn observe_initial_chunk_material(packet: &horizon_protocol::packets::play::LevelChunkWithLight) {
    use horizon_world::nbt::Tag;
    if (packet.x, packet.z) != (0, 0) { return; }
    for entity in &packet.chunk.block_entities {
        if entity.packed_xz == 2 && entity.y == 71 {
            if let Some(Tag::String(material)) = entity.tag.get("Material") {
                eprintln!("TCONSTRUCT_WIRE_CHUNK {}", material.to_string_lossy());
            }
        }
    }
}
