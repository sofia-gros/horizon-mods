fn send_part_selection_data(hub: &mut Hub, ctx: &mut TickCtx<'_>) {
    let world = &*ctx.world;
    for i in 0..hub.guests.len() {
        if !hub.here(&hub.guests[i]) { continue; }
        let Some(menu) = hub.guests[i].menu.as_ref() else { continue; };
        let Some(pos) = menu.pos else { continue; };
        if !menu.slots.valid(world) { continue; }
        let state = world.block(pos.0, pos.1, pos.2).0 as usize;
        let builder = hub.sync.builtin.get("minecraft:block").and_then(|r| r.id("tconstruct:part_builder"));
        if builder.is_none() || hub.encoder.block_ids.get(state).copied() != builder { continue; }
        let has_pattern = world.block_entities.get(&pos).and_then(|entity| entity.slots().first())
            .and_then(Option::as_ref).is_some_and(|stack| &*stack.item == "tconstruct:pattern");
        let value = if has_pattern {
            world.chunks.__horizon_mixins_tconstruct_part_selections().get(&pos).and_then(|s| crate::tconstruct_tx::preview_pattern_index(s)).unwrap_or(-1)
        } else { -1 };
        let guest = &mut hub.guests[i];
        let Some(menu) = guest.menu.as_mut() else { continue; };
        menu.sent_data.resize(1, i16::MIN);
        if menu.sent_data[0] == value { continue; }
        menu.sent_data[0] = value;
        let window = menu.window;
        guest.send(&C::ContainerSetData(play::ContainerSetData { container: window, id: 0, value }));
    }
}
