use super::{Hub, World, Rules, ItemStack, View, menu, play};

pub(super) fn pickup(hub: &mut Hub, gi: usize, packet: &play::ContainerClick, world: &mut World, rules: &dyn Rules,
    pos: (i32,i32,i32), mut state: crate::tconstruct_source_smeltery_state::Smeltery, offset: usize,
    containers: [Option<ItemStack>; 2]) -> Vec<ItemStack> {
    // Native Pickup mutates the guest before Slots::write, whose API cannot
    // report failure. Own this transaction so a rejected source commit restores
    // inventory, cursor and drag and suppresses all speculative drops.
    let expected = crate::tconstruct_source_smeltery_page::project_with_containers(&state.slots, offset, &containers);
    let mut items = expected.clone();
    let previous_inventory = hub.guests[gi].inventory.clone();
    let previous_carried = hub.guests[gi].carried.clone();
    let mut drag = hub.guests[gi].menu.as_ref().and_then(|menu| menu.drag.clone());
    let hidden = hub.guests[gi].menu.as_ref().map(|menu| menu.hidden.clone()).unwrap_or_default();
    let (drops, changed) = {
        let mut view = View { guest: &mut hub.guests[gi], container: Some(&mut items), shulker: false, hidden: &hidden };
        menu::click(&mut view, &mut drag, i32::from(packet.slot), i32::from(packet.button), packet.input, rules)
    };
    let before = state.slots.clone();
    let committed = crate::tconstruct_source_smeltery_page::no_retained_wire(&items)
        && crate::tconstruct_source_smeltery_page::commit_with_containers(&mut state.slots, offset, &expected, &items, &containers);
    if committed {
        for (index, (old, new)) in before.iter().zip(&state.slots).enumerate() {
            let same = match (old, new) {
                (None, None) => true,
                (Some(a), Some(b)) => a.item == b.item && a.damage == b.damage && a.extra == b.extra,
                _ => false,
            };
            if !same { state.progress[index] = 0; }
        }
        if crate::tconstruct_source_melter_store::write(world, pos, "tconstruct:smeltery", state.to_nbt()) {
            if let Some(menu) = hub.guests[gi].menu.as_mut() { menu.drag = drag; }
            hub.answer_click(gi, packet, world, &changed);
            return drops;
        }
    }
    hub.guests[gi].inventory = previous_inventory;
    hub.guests[gi].carried = previous_carried;
    let current = crate::tconstruct_source_melter_store::read(world, pos, "tconstruct:smeltery")
        .and_then(|tag| crate::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag));
    if let Some(current) = current {
        hub.send_menu_content(gi, crate::tconstruct_source_smeltery_page::project_with_containers(&current.slots, offset, &containers));
    }
    Vec::new()
}
