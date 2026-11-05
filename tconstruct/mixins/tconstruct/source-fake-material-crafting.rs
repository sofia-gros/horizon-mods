// Material-preserving fake ingot / storage-block conversions from ToolsRecipeProvider.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

/// Plan only: the caller must pass the world-resolved eligibility of the output
/// material. No input is changed, and the host retains its normal remainders.
pub(super) fn plan(stacks: &[&ItemStack], width: usize, height: usize, to_block: bool,
                   eligible: impl FnOnce(&str, &str) -> bool) -> Option<ItemStack> {
    let (input, output, count, slots) = if to_block {
        if width != 3 || height != 3 { return None; }
        ("tconstruct:fake_ingot", "tconstruct:fake_storage_block", 1, 9)
    } else {
        if width != 1 || height != 1 { return None; }
        ("tconstruct:fake_storage_block", "tconstruct:fake_ingot", 9, 1)
    };
    if stacks.len() != slots { return None; }
    let mut material: Option<String> = None;
    for stack in stacks {
        if &*stack.item != input || stack.count == 0 { return None; }
        let Tag::String(id) = stack.data()?.other("minecraft:custom_data")?.get("Material")? else { return None; };
        let id = id.to_string_lossy();
        if material.as_deref().is_some_and(|old| old != id) { return None; }
        material = Some(id);
    }
    let material = material?;
    if !eligible(&material, output) { return None; }
    let mut result = ItemStack::new(output, count);
    result.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
        ("Material", Tag::String(material.as_str().into()))
    ])));
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stack(item: &str, material: &str, count: u32) -> ItemStack {
        let mut s = ItemStack::new(item, count);
        s.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("Material", Tag::String(material.into()))
        ])));
        s
    }
    #[test]
    fn nine_matching_ingots_and_block_round_trip_material() {
        let items: Vec<_> = (0..9).map(|_| stack("tconstruct:fake_ingot", "test:bronze", 2)).collect();
        let refs: Vec<_> = items.iter().collect();
        let block = plan(&refs, 3, 3, true, |m, out| m == "test:bronze" && out == "tconstruct:fake_storage_block").unwrap();
        assert_eq!(block.count, 1);
        let ingots = plan(&[&block], 1, 1, false, |m, out| m == "test:bronze" && out == "tconstruct:fake_ingot").unwrap();
        assert_eq!(ingots.count, 9);
        assert_eq!(ingots.data().unwrap().other("minecraft:custom_data").unwrap().get("Material"), items[0].data().unwrap().other("minecraft:custom_data").unwrap().get("Material"));
        assert!(items.iter().all(|s| s.count == 2));
    }
    #[test]
    fn plan_keeps_input_components_untouched() {
        let mut block = stack("tconstruct:fake_storage_block", "test:bronze", 2);
        block.data_mut().set_other("minecraft:custom_name", Some(Tag::String("kept".into())));
        let original = block.clone();
        let result = plan(&[&block], 1, 1, false, |_, _| true).unwrap();
        assert_eq!(block.count, original.count);
        assert!(block.same_item_same_components(&original));
        // Source MaterialsConsumer sets Material on a fresh recipe result.
        assert_eq!(result.count, 9);
        assert!(result.data().unwrap().other("minecraft:custom_name").is_none());
    }
    #[test]
    fn mixed_missing_and_ineligible_inputs_never_mutate() {
        let mut items: Vec<_> = (0..9).map(|_| stack("tconstruct:fake_ingot", "test:bronze", 1)).collect();
        assert!(plan(&items.iter().collect::<Vec<_>>()[..8], 3, 3, true, |_,_| true).is_none());
        assert!(plan(&items.iter().collect::<Vec<_>>(), 3, 3, true, |_,_| false).is_none());
        items[8] = stack("tconstruct:fake_ingot", "test:silver", 1);
        assert!(plan(&items.iter().collect::<Vec<_>>(), 3, 3, true, |_,_| true).is_none());
        assert!(items.iter().all(|s| s.count == 1));
    }
    #[test]
    fn conversion_geometry_empty_stacks_and_missing_material_fail_closed() {
        let mut ingots: Vec<_> = (0..9).map(|_| stack("tconstruct:fake_ingot", "test:bronze", 1)).collect();
        assert!(plan(&ingots.iter().collect::<Vec<_>>(), 1, 9, true, |_,_| true).is_none());
        ingots[4].count = 0;
        assert!(plan(&ingots.iter().collect::<Vec<_>>(), 3, 3, true, |_,_| true).is_none());
        ingots[4] = ItemStack::new("tconstruct:fake_ingot", 1);
        assert!(plan(&ingots.iter().collect::<Vec<_>>(), 3, 3, true, |_,_| true).is_none());
        let block = stack("tconstruct:fake_storage_block", "test:bronze", 1);
        assert!(plan(&[&block], 2, 1, false, |_,_| true).is_none());
        assert!(plan(&[&block, &block], 1, 1, false, |_,_| true).is_none());
    }
}
