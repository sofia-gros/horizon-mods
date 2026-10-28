fn virtual_part_selection(hub: &crate::lan::hub::Hub, index: usize, world: &World, slot: usize) -> std::ops::ControlFlow<Option<ItemStack>> {
    use std::ops::ControlFlow::{Break, Continue};
    if slot != 23 { return Continue(()); }
    let Some(pos) = hub.guests.get(index).and_then(|g| g.menu.as_ref()).and_then(|m| m.pos) else { return Continue(()); };
    let state = world.block(pos.0, pos.1, pos.2).0 as usize;
    let builder = hub.sync.builtin.get("minecraft:block").and_then(|r| r.id("tconstruct:part_builder"));
    if builder.is_none() || hub.encoder.block_ids.get(state).copied() != builder { return Continue(()); }
    let item = match world.chunks.__horizon_mixins_tconstruct_part_selections().get(&pos) {
        Some(0) => Some("tconstruct:flint_pick_head"),
        Some(1) => Some("tconstruct:wood_handle"),
        Some(2) => Some("tconstruct:wood_binding"),
        _ => None,
    };
    Break(item.map(|id| ItemStack::new(id, 1)))
}
