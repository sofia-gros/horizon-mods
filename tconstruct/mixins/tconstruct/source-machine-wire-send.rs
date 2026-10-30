// Send only changed machine metadata to guests tracking its chunk. Inventories
// continue to use the existing menu protocol.
fn tconstruct_machine_wire_send(hub: &mut crate::lan::hub::Hub, ctx: &mut crate::lan::hub::TickCtx<'_>) {
    use horizon_world::nbt::{Compound, Tag};
    use std::collections::HashMap;
    let mut live = HashMap::new();
    for (_, chunk) in &ctx.world.chunks {
        let Some(saved) = chunk.saved.as_deref() else { continue; };
        for (pos, raw) in &saved.block_entity_sources {
            let Some(Tag::String(id)) = raw.get("id") else { continue; };
            let id = id.to_string_lossy();
            let block = match id.as_str() {
                "tconstruct:melter" => "tconstruct:seared_melter",
                "tconstruct:heater" => "tconstruct:seared_heater",
                "tconstruct:faucet" => "tconstruct:seared_faucet",
                "tconstruct:table" => "tconstruct:seared_table",
                "tconstruct:basin" => "tconstruct:seared_basin",
                _ => continue,
            };
            let Some(kind) = hub.sync.builtin.get("minecraft:block_entity_type").and_then(|r| r.id(&id)) else { continue; };
            let Some(block_id) = hub.sync.builtin.get("minecraft:block").and_then(|r| r.id(block)) else { continue; };
            let state = ctx.world.block(pos.0, pos.1, pos.2).0 as usize;
            if hub.encoder.block_ids.get(state).copied() != Some(block_id) { continue; }
            let Some(value) = raw.get(id.as_str()) else { continue; };
            let mut data = Compound::new(); data.insert(id.into(), value.clone());
            live.insert([pos.0, pos.1, pos.2], (kind, Tag::Compound(data)));
        }
    }
    let cache = ctx.world.chunks.__horizon_mixins_tconstruct_machine_wire_sent_mut();
    cache.retain(|(conn, joined, pos), _| live.contains_key(pos) && hub.guests.iter().any(|g|
        g.conn == *conn && g.placed_at == *joined && hub.here(g) && g.sent_chunks.contains(&(pos[0] >> 4, pos[2] >> 4))));
    for guest in &hub.guests {
        if !hub.here(guest) { continue; }
        for (pos, (kind, tag)) in &live {
            if !guest.sent_chunks.contains(&(pos[0] >> 4, pos[2] >> 4)) { continue; }
            let key = (guest.conn, guest.placed_at, *pos);
            if cache.get(&key) == Some(tag) { continue; }
            guest.send(&C::BlockEntityData(play::BlockEntityData { pos: *pos, kind: *kind, tag: tag.clone() }));
            cache.insert(key, tag.clone());
        }
    }
}
