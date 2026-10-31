fn smeltery_guest_write(slots: crate::lan::menu::Slots, world: &mut crate::world::World, guest: &crate::lan::guest::Guest, items: &[Option<ItemStack>]) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let crate::lan::menu::Slots::Block(pos) = slots else { return Continue(()); };
    let Some(tag) = crate::tconstruct_source_melter_store::read(world, pos, "tconstruct:smeltery") else { return Continue(()); };
    let Some(mut state) = crate::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag) else { return Break(()); };
    let offset = world.chunks.__horizon_mixins_tconstruct_smeltery_guest_pages().get(&(guest.conn, guest.placed_at, pos)).map_or(0, |page| page.0);
    let previous = state.slots.clone();
    let expected = crate::tconstruct_source_smeltery_page::project(&previous, offset);
    if !crate::tconstruct_source_smeltery_page::commit(&mut state.slots, offset, &expected, items) { return Break(()); }
    for (index, (before, after)) in previous.iter().zip(&state.slots).enumerate() {
        let same = match (before, after) {
            (None, None) => true,
            (Some(a), Some(b)) => a.item == b.item && a.damage == b.damage && a.extra == b.extra,
            _ => false,
        };
        if !same { state.progress[index] = 0; }
    }
    let _ = crate::tconstruct_source_melter_store::write(world, pos, "tconstruct:smeltery", state.to_nbt());
    Break(())
}
