#[path = "../../mixins/tconstruct/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path = "../../mixins/tconstruct/source-overslime.rs"] mod tconstruct_source_overslime;
#[path = "../../mixins/tconstruct/source-overslime-crafting.rs"] mod crafting;

#[test]
fn source_bottle_craft_preserves_unused_and_returns_glass() {
    use horizon_items::items::ItemStack;
    use horizon_world::nbt::Tag;
    let mut tool=ItemStack::new("tconstruct:pickaxe",1);
    tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
        ("tic_overslime",Tag::Int(30)),("tic_overslime_capacity",Tag::Int(50)),
        ("tic_overslime_active",Tag::Byte(1)),("tic_overslime_overworked",Tag::Int(0)),
    ])));
    let bottle=ItemStack::new("tconstruct:earth_slime_bottle",1);
    let grid=[Some(tool),Some(bottle.clone()),Some(bottle)];
    let (result,remaining)=crafting::plan(&grid).unwrap();
    assert_eq!(tconstruct_source_overslime::amount(&result,"tic_overslime"),50);
    assert_eq!(remaining[1].as_ref().unwrap().item.as_ref(),"minecraft:glass_bottle");
    assert_eq!(remaining[2].as_ref().unwrap().item.as_ref(),"tconstruct:earth_slime_bottle");
    let mut mixed=grid.to_vec();mixed[2]=Some(ItemStack::new("tconstruct:sky_slime_bottle",1));
    assert!(crafting::plan(&mixed).is_none());
}
