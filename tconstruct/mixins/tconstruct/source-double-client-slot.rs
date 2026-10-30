fn tconstruct_double_client_slot(game: &mut crate::game::Game<'_>, container: i32, slot: i32, item: &Option<ItemStack>) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote_menu_title().map(|title| title.plain()).as_deref() != Some("TConstruct Part Builder Double") || game.remote_menu_id() != container { return Continue(()); }
    let Some(key) = game.container else { return Break(()); };
    if let Some(BlockEntity::Chest(items)) = game.world.block_entities.get_mut(&key) { items.resize(81, None); }
    let internal = match slot {
        0..=80 => 300 + slot as usize,
        81..=107 => 9 + (slot - 81) as usize,
        108..=116 => (slot - 108) as usize,
        _ => return Break(()),
    };
    *game.slot_mut(internal) = item.clone();
    Break(())
}
