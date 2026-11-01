#[path = "../../mixins/tconstruct/source-melter-state.rs"] mod state;
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

#[test]
fn pending_material_survives_save_and_output_is_atomic() {
    let mut part = ItemStack::new("tconstruct:pick_head", 1);
    part.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("Material", Tag::String("tconstruct:cobalt".into()))])));
    let mut table = state::Casting::new();
    table.input = Some(ItemStack::new("tconstruct:pick_head_cast", 1));
    table.output = Some(ItemStack::new("minecraft:stone", 1));
    assert!(!table.prepare("test:pick_head", 180, 2, part.clone(), false, false));
    assert_eq!(table.tank.amount, 0);
    table.output = None;
    assert!(table.prepare("test:pick_head", 180, 2, part, false, false));
    assert!(table.tank.insert("tconstruct:molten_cobalt", 180));
    let mut table = state::Casting::from_nbt(&table.to_nbt()).unwrap();
    assert!(!table.tick(false));
    assert_eq!(table.tank.amount, 180);
    assert!(!table.tick(true)); assert!(table.tick(true));
    assert_eq!(table.output.as_ref().unwrap().data().unwrap().other("minecraft:custom_data").unwrap().get("Material"), Some(&Tag::String("tconstruct:cobalt".into())));
    assert_eq!(&*table.input.unwrap().item, "tconstruct:pick_head_cast");
}
