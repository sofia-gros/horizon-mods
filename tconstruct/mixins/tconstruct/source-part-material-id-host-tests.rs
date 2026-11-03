#[cfg(test)]
mod tests {
    use horizon_items::{item_data::parse_stack, item_nbt::{item_from_nbt, item_to_nbt}};
    use horizon_world::nbt::Tag;

    #[test]
    fn material_identity_survives_stack_nbt_roundtrip() {
        let stack = parse_stack(r#"{id:"tconstruct:pick_head",count:1,components:{"minecraft:custom_data":{Material:"tconstruct:copper#oxidized"}}}"#).unwrap().unwrap();
        let restored = item_from_nbt(&item_to_nbt(&stack)).unwrap();
        assert!(stack.same_item_same_components(&restored));
        let Some(Tag::String(raw)) = restored.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|data| data.get("Material")) else { panic!("material lost") };
        assert_eq!(super::super::tconstruct_source_part_material_id::source_part_material_id(&restored.item, &raw.to_string_lossy(), true, false, false).as_deref(), Some("tconstruct:copper#oxidized"));
    }
}
