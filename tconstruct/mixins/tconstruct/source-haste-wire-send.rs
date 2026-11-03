// Send each actor only the Haste trait value for that actor's current chest.
fn tconstruct_haste_wire_send(hub: &mut crate::lan::hub::Hub, ctx: &mut crate::lan::hub::TickCtx<'_>) {
    let generation = *ctx.world.chunks.__horizon_mixins_tconstruct_source_recipe_generation();
    // Invalidated snapshots must replace a cached positive result immediately,
    // even before the next successful capture increments the generation.
    let ready = ctx.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
        .and_then(|v| v.as_ref().ok()).and_then(|tags| tags.get("tconstruct:modifiable/harvest")).is_some()
        && ctx.world.chunks.__horizon_mixins_tconstruct_material_snapshot().is_some()
        && ctx.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().is_some()
        && ctx.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
            .and_then(|v| v.as_ref().ok()).and_then(|raw| raw.0.get("tconstruct:tinkering/modifiers/haste"))
            .and_then(|layers| layers.last())
            .is_some_and(|source| crate::tconstruct_source_haste_source::source_haste_trait_allowed(Some(false), Some(source)));
    ctx.world.chunks.__horizon_mixins_tconstruct_haste_wire_sent_mut().retain(|(conn, joined), _|
        hub.guests.iter().any(|guest| guest.conn == *conn && guest.placed_at == *joined && hub.here(guest)));
    for guest in &hub.guests {
        if !hub.here(guest) { continue; }
        let key = (guest.conn, guest.placed_at);
        let chest = guest.armor[1].clone();
        let cached = ctx.world.chunks.__horizon_mixins_tconstruct_haste_wire_sent().get(&key).cloned();
        let changed = cached.as_ref().is_none_or(|(old_generation, old_ready, old_chest, _)| *old_generation != generation || *old_ready != ready || *old_chest != chest);
        let bytes = if changed {
            let value = (|| {
                if !ready { return None; }
                let stack = chest.as_ref()?;
                let tags = ctx.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
                let harvest = tags.get("tconstruct:modifiable/harvest")?;
                let raw = ctx.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()?.as_ref().ok()?;
                let source = raw.0.get("tconstruct:tinkering/modifiers/haste")?.last()?.as_slice();
                if !crate::tconstruct_source_haste_source::source_haste_trait_allowed(Some(harvest.contains(&*stack.item)), Some(source)) { return None; }
                let levels = crate::tconstruct_source_material_traits::levels(&*ctx.world, stack)?;
                Some((stack, levels.levels.get("tconstruct:haste").copied().unwrap_or(0)))
            })();
            crate::lan::hub::tconstruct_source_haste_wire::encode(value)
        } else { cached.as_ref().unwrap().3.0.clone() };
        let cache = ctx.world.chunks.__horizon_mixins_tconstruct_haste_wire_sent_mut();
        let entry = cache.entry(key).or_insert_with(|| (generation, ready, chest.clone(), (Vec::new(),0)));
        entry.0 = generation; entry.1 = ready; entry.2 = chest;
        if !crate::lan::hub::tconstruct_source_haste_wire::needs_send(&mut entry.3, &bytes) { continue; }
        guest.send(&C::CustomPayload(horizon_protocol::packets::configuration::CustomPayload {
            channel: "tconstruct:haste_chest".into(), data: bytes,
        }));
    }
}
