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
            let progress = match value.get("progress") { Some(Tag::IntArray(values)) if id == "tconstruct:smeltery" => Some(values.clone()), _ => None };
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
            live.insert([pos.0, pos.1, pos.2], (kind, Tag::Compound(data), progress));
        }
    }
    let pages = ctx.world.chunks.__horizon_mixins_tconstruct_smeltery_guest_pages().clone();
    let cache = ctx.world.chunks.__horizon_mixins_tconstruct_machine_wire_sent_mut();
    cache.retain(|(conn, joined, pos), _| live.contains_key(pos) && hub.guests.iter().any(|g|
        g.conn == *conn && g.placed_at == *joined && hub.here(g) && g.sent_chunks.contains(&(pos[0] >> 4, pos[2] >> 4))));
    for guest in &hub.guests {
        if !hub.here(guest) { continue; }
        for (pos, (kind, base, progress)) in &live {
            if !guest.sent_chunks.contains(&(pos[0] >> 4, pos[2] >> 4)) { continue; }
            let mut tag = base.clone();
            if let Some(progress) = progress {
                let at = (pos[0], pos[1], pos[2]);
                if let Some((offset, count)) = pages.get(&(guest.conn, guest.placed_at, at)).copied() {
                    let columns = count.div_ceil(7).min(4);
                    let start = offset.saturating_mul(columns).min(progress.len());
                    let end = start.saturating_add(40).min(progress.len());
                    if let Tag::Compound(fields) = &mut tag {
                        if let Some(Tag::Compound(machine)) = fields.get_mut(&"tconstruct:smeltery".into()) {
                            machine.insert("progress_start".into(), Tag::Int(start as i32));
                            machine.insert("progress_page".into(), Tag::IntArray(progress[start..end].to_vec()));
                        }
                    }
                }
            }
            let key = (guest.conn, guest.placed_at, *pos);
            if cache.get(&key) == Some(&tag) { continue; }
            guest.send(&C::BlockEntityData(play::BlockEntityData { pos: *pos, kind: *kind, tag: tag.clone() }));
            cache.insert(key, tag);
        }
    }
}
