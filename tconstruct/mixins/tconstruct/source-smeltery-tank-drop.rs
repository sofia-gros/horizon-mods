fn smeltery_tank_drop(pending: &mut crate::game::spill::Pending, pos: [i32; 3], stack: &mut horizon_client::items::ItemStack) {
    use horizon_world::nbt::{Compound, Tag};
    if &*stack.item == "tconstruct:tinkers_chest" {
        if let Some((item,patch))=pending.copied.get_mut(&pos) {
            if item==&*stack.item {
                if let Some(Tag::Int(color))=patch.remove(&"tconstruct:internal_chest_color".into()).and_then(|tag|tag.get("color").cloned()) {
                    stack.data_mut().dyed_color=Some(color);
                }
            }
        }
        return;
    }
    if matches!(&*stack.item, "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" | "tconstruct:modifier_worktable") {
        if let Some((item, patch)) = pending.copied.get_mut(&pos) {
            if item == &*stack.item {
                if let Some(value) = patch.remove(&"tconstruct:internal_anvil_copy".into()) {
                    let mut data = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
                        Some(Tag::Compound(data)) => data.clone(), _ => Compound::new(),
                    };
                    if let Some(Tag::String(metal)) = value.get("Variant") {
                        let model = format!("{}_{}", stack.item, metal.to_string_lossy());
                        stack.data_mut().set_other("minecraft:item_model", Some(Tag::String(model.as_str().into())));
                    }
                    data.insert("tconstruct:anvil".into(), value);
                    stack.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(data)));
                }
            }
        }
        return;
    }
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
