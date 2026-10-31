fn smeltery_tank_drop(pending: &mut crate::game::spill::Pending, pos: [i32; 3], stack: &mut horizon_client::items::ItemStack) {
    use horizon_world::nbt::{Compound, Tag};
    let marker = "tconstruct:internal_tank_copy";
    let Some((item, patch)) = pending.copied.get_mut(&pos) else { return; };
    if item != &*stack.item || crate::game::tconstruct_source_smeltery_fuel::capacity(&stack.item).is_none() { return; }
    let Some(tank) = patch.remove(&marker.into()) else { return; };
    let mut data = match stack.data().and_then(|data| data.other("minecraft:custom_data")) {
        Some(Tag::Compound(data)) => data.clone(), _ => Compound::new(),
    };
    data.insert("tconstruct:tank".into(), tank);
    stack.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(data)));
    if patch.is_empty() { pending.copied.remove(&pos); }
}
