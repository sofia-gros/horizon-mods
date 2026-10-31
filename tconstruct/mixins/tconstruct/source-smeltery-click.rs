fn smeltery_click(game: &mut crate::game::Game<'_>, slot: Option<usize>, button: i32, input: horizon_client::menu::Input) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    if !crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) { return Continue(()); }
    if input != horizon_client::menu::Input::Pickup { return Break(true); }
    let (count, offset) = crate::game::tconstruct_source_smeltery_ui::page(game);
    // Reject outside clicks and every synthetic container slot before the
    // generic menu can alter the carried stack or player inventory.
    let Some(slot) = slot else { return Break(true); };
    if (36..300).contains(&slot) { return Break(true); }
    if slot >= 300 && (slot >= 354 || crate::game::tconstruct_source_smeltery_layout::melting_index(count, offset, slot - 300).is_none()) { return Break(true); }
    if game.remote.is_some() { return Continue(()); }
    let Some(pos) = game.container else { return Break(true); };
    let Some(mut state) = crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
        .and_then(|tag| crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) else { return Break(true); };
    let expected = crate::game::tconstruct_source_smeltery_page::project(&state.slots, offset);
    let Some(horizon_client::containers::BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) else { return Break(true); };
    let previous_live = live.clone();
    *live = expected.clone();
    let previous_inventory = game.inventory.clone();
    let previous_carried = game.carried.clone();
    let handled = game.menu_ops(&[(Some(slot), button, input)]);
    let changed = game.world.block_entities.get(&pos).map(|entity| entity.slots().to_vec());
    let before_slots = state.slots.clone();
    let committed = changed.as_ref().is_some_and(|view| crate::game::tconstruct_source_smeltery_page::commit(&mut state.slots, offset, &expected, view));
    if committed {
        for (index, (before, after)) in before_slots.iter().zip(&state.slots).enumerate() {
            let same = match (before, after) {
                (None, None) => true,
                (Some(a), Some(b)) => a.item == b.item && a.damage == b.damage && a.extra == b.extra,
                _ => false,
            };
            if !same { state.progress[index] = 0; }
        }
        if crate::game::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", state.to_nbt()) { return Break(handled); }
    }
    game.inventory = previous_inventory;
    game.carried = previous_carried;
    if let Some(horizon_client::containers::BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) { *live = previous_live; }
    Break(true)
}
