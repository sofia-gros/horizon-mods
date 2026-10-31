fn smeltery_guest_guard(hub: &mut crate::lan::hub::Hub, conn: u32, packet: &play::ContainerClick, world: &mut World) -> std::ops::ControlFlow<Vec<ItemStack>> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(gi) = hub.guests.iter().position(|guest| guest.conn == conn) else { return Continue(()); };
    let Some(menu) = hub.guests[gi].menu.as_ref() else { return Continue(()); };
    if packet.container == 0 || menu.window != packet.container { return Continue(()); }
    let Some(pos) = menu.pos else { return Continue(()); };
    let Some(tag) = crate::tconstruct_source_melter_store::read(world, pos, "tconstruct:smeltery") else { return Continue(()); };
    let Some(state) = crate::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag) else { return Break(Vec::new()); };
    let key = (conn, hub.guests[gi].placed_at, pos);
    let offset = world.chunks.__horizon_mixins_tconstruct_smeltery_guest_pages().get(&key).copied().unwrap_or(0);
    // Until quick-move and bucket slots have a real transaction, reject them
    // before the generic menu can mutate inventory or a virtual slot.
    let virtual_slot = (0..54).contains(&packet.slot) && crate::tconstruct_source_smeltery_layout::melting_index(state.slots.len(), offset, packet.slot as usize).is_none();
    if packet.input != 0 || virtual_slot {
        hub.send_menu_content(gi, crate::tconstruct_source_smeltery_page::project(&state.slots, offset));
        return Break(Vec::new());
    }
    Continue(())
}
