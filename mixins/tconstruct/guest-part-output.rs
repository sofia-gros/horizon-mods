fn guest_tconstruct_part_output(hub: &mut crate::lan::hub::Hub, conn: u32, packet: &play::ContainerClick, world: &mut World) -> std::ops::ControlFlow<Vec<ItemStack>> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(gi) = hub.guests.iter().position(|g| g.conn == conn) else { return Continue(()); };
    let Some(menu) = hub.guests[gi].menu.as_ref() else { return Continue(()); };
    if packet.container == 0 || menu.window != packet.container || hub.guests[gi].spectator() || !menu.slots.still_valid(world, &hub.guests[gi], menu.pos) { return Continue(()); }
    let Some(key) = menu.pos else { return Continue(()); };
    let state = world.block(key.0, key.1, key.2).0 as usize;
    let actual = hub.encoder.block_ids.get(state).copied();
    let block = hub.sync.builtin.get("minecraft:block");
    let builder = block.and_then(|r| r.id("tconstruct:part_builder"));
    let station = block.and_then(|r| r.id("tconstruct:tinker_station"));
    let is_station = if builder.is_some() && actual == builder { false } else if station.is_some() && actual == station { true } else { return Continue(()); };
    if packet.input != 0 || ((2..=23).contains(&packet.slot) && (!is_station || packet.slot != 2)) {
        if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
        return Break(Vec::new());
    }
    if !(24..=26).contains(&packet.slot) { return Continue(()); }
    if packet.button == 0 {
        if let Some(BlockEntity::Chest(stacks)) = world.block_entities.get_mut(&key) {
            if crate::tconstruct_tx::take(stacks, &mut hub.guests[gi].carried, is_station, packet.slot) {
                world.modified.insert((key.0 >> 4, key.2 >> 4));
            }
        }
    }
    if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
    Break(Vec::new())
}
