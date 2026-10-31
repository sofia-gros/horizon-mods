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
            let blocks: &[&str] = match id.as_str() {
                "tconstruct:melter" => &["tconstruct:seared_melter"],
                "tconstruct:heater" => &["tconstruct:seared_heater"],
                "tconstruct:faucet" => &["tconstruct:seared_faucet"],
                "tconstruct:table" => &["tconstruct:seared_table"],
                "tconstruct:basin" => &["tconstruct:seared_basin"],
                "tconstruct:smeltery" => &["tconstruct:smeltery_controller", "tconstruct:foundry_controller"],
                "tconstruct:fuel_tank" => &["tconstruct:seared_fuel_tank", "tconstruct:seared_fuel_gauge", "tconstruct:scorched_fuel_tank", "tconstruct:scorched_fuel_gauge"],
                "tconstruct:ingot_tank" => &["tconstruct:seared_ingot_tank", "tconstruct:seared_ingot_gauge", "tconstruct:scorched_ingot_tank", "tconstruct:scorched_ingot_gauge"],
                _ => continue,
            };
            let Some(kind) = hub.sync.builtin.get("minecraft:block_entity_type").and_then(|r| r.id(&id)) else { continue; };
            let state = ctx.world.block(pos.0, pos.1, pos.2).0 as usize;
            let Some(block_id) = hub.encoder.block_ids.get(state).copied() else { continue; };
            if !blocks.iter().any(|block| hub.sync.builtin.get("minecraft:block").and_then(|r| r.id(block)) == Some(block_id)) { continue; }
            let Some(value) = raw.get(id.as_str()) else { continue; };
            let mut value = value.clone();
            // Phase is only the server's scheduling cursor. Sending it would
            // broadcast an otherwise unchanged Melter every tick.
            if matches!(id.as_str(), "tconstruct:melter" | "tconstruct:smeltery") {
                if let Tag::Compound(fields) = &mut value { fields.insert("phase".into(), Tag::Int(0)); }
            }
            if id == "tconstruct:smeltery" {
                let count = match value.get("slots") { Some(Tag::List(slots)) => slots.len(), _ => continue };
                if let Tag::Compound(fields) = &mut value {
                    fields.retain(|key, _| !matches!(key.to_string_lossy().as_str(), "slots" | "progress"));
                    fields.insert("slot_count".into(), Tag::Int(count as i32));
                }
            }
            let mut data = Compound::new(); data.insert(id.as_str().into(), value);
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
