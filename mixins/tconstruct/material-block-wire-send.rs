fn material_block_wire_send(hub: &mut crate::lan::hub::Hub, ctx: &mut crate::lan::hub::TickCtx<'_>) {
    use horizon_world::nbt::{Compound, Tag};
    use std::collections::HashMap;
    let Some(kind) = hub.sync.builtin.get("minecraft:block_entity_type").and_then(|r| r.id("tconstruct:material_block")) else {
        ctx.world.chunks.__horizon_mixins_tconstruct_material_wire_sent_mut().clear();
        return;
    };
    let Some(block) = hub.sync.builtin.get("minecraft:block").and_then(|r| r.id("tconstruct:fake_storage_block")) else {
        ctx.world.chunks.__horizon_mixins_tconstruct_material_wire_sent_mut().clear();
        return;
    };
    let mut live = HashMap::<[i32; 3], Tag>::new();
    for (_, chunk) in &ctx.world.chunks {
        let Some(saved) = chunk.saved.as_deref() else { continue; };
        for raw in &saved.block_entities {
            if raw.get("id") != Some(&Tag::String("tconstruct:material_block".into())) { continue; }
            let pos = match (raw.get("x"), raw.get("y"), raw.get("z")) {
                (Some(Tag::Int(x)), Some(Tag::Int(y)), Some(Tag::Int(z))) => [*x, *y, *z],
                _ => continue,
            };
            let state = ctx.world.block(pos[0], pos[1], pos[2]).0 as usize;
            if hub.encoder.block_ids.get(state).copied() != Some(block) { continue; }
            let mut data = Compound::new();
            if let Some(material) = raw.get("Material") { data.insert("Material".into(), material.clone()); }
            live.insert(pos, Tag::Compound(data));
        }
    }
    let cache = ctx.world.chunks.__horizon_mixins_tconstruct_material_wire_sent_mut();
    cache.retain(|(conn, joined, pos), _| live.contains_key(pos) && hub.guests.iter().any(|g|
        g.conn == *conn && g.placed_at == *joined && hub.here(g) && g.sent_chunks.contains(&(pos[0] >> 4, pos[2] >> 4))));
    for guest in &hub.guests {
        if !hub.here(guest) { continue; }
        for (pos, tag) in &live {
            if !guest.sent_chunks.contains(&(pos[0] >> 4, pos[2] >> 4)) { continue; }
            let key = (guest.conn, guest.placed_at, *pos);
            if cache.get(&key) == Some(tag) { continue; }
            guest.send(&C::BlockEntityData(play::BlockEntityData { pos: *pos, kind, tag: tag.clone() }));
            cache.insert(key, tag.clone());
        }
    }
}
