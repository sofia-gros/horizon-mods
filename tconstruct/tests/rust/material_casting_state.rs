include!("pluto_storage_fixture.rs");
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

#[test]
fn composite_consumes_input_only_after_valid_completed_pour() {
    let part = |id: &str| {
        let mut stack = ItemStack::new("tconstruct:pick_head", 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("Material", Tag::String(id.into()))])));
        stack
    };
    let mut table = state::Casting::new();
    table.input = Some(part("tconstruct:copper"));
    table.output = Some(ItemStack::new("minecraft:stone", 1));
    assert!(!table.prepare("test:oxidize", 500, 2, part("tconstruct:copper#oxidized"), true, false));
    assert_eq!(table.input.as_ref().unwrap().count, 1);
    table.output = None;
    assert!(table.prepare("test:oxidize", 500, 2, part("tconstruct:copper#oxidized"), true, false));
    assert!(table.tank.insert("minecraft:water", 500));
    let mut table = state::Casting::from_nbt(&table.to_nbt()).unwrap();
    assert!(!table.tick(false));
    assert!(table.input.is_some()); assert_eq!(table.tank.amount, 500);
    assert!(!table.tick(true)); assert!(table.tick(true));
    assert!(table.input.is_none()); assert_eq!(table.tank.amount, 0);
    assert_eq!(table.output.unwrap().data().unwrap().other("minecraft:custom_data").unwrap().get("Material"), Some(&Tag::String("tconstruct:copper#oxidized".into())));
}
