use super::*;
use horizon_items::item_data::Wire;
use horizon_world::nbt::JavaString;
use pluto_inventory_api::crafting::matching::Outcome;
use std::sync::Once;
const CUSTOM: &str = "minecraft:custom_data";
fn stack(id: &str, custom: Option<Tag>) -> ItemStack {
    let mut stack = ItemStack::new(id, 1);
    if let Some(custom) = custom { stack.data_mut().other.push((CUSTOM.into(), custom)); }
    stack
}
fn fake(material: Tag) -> ItemStack {
    stack("tconstruct:fake_storage_block", Some(Tag::compound([("Material", material)])))
}
fn expected(material: Tag) -> ItemStack {
    stack("tconstruct:tinkers_anvil", Some(Tag::compound([("tconstruct:anvil", Tag::compound([("RequiredMaterial", material)]))])))
}
fn template(stack: ItemStack) -> Template {
    Template { item: Some(stack.item), count: stack.count, damage: stack.damage, extra: stack.extra, components: None }
}
fn shaped(result: ItemStack) -> Recipe {
    Recipe::Shaped { width: 3, height: 3, cells: vec![None; 9], result: template(result) }
}
fn grid(stacks: Vec<ItemStack>) -> Vec<Option<ItemStack>> {
    assert!(stacks.len() <= 9);
    let mut grid: Vec<_> = stacks.into_iter().map(Some).collect(); grid.resize(9, None); grid
}
fn material() -> Tag { Tag::String("example:iron".into()) }
fn compare(recipe: &Recipe, grid: &[Option<ItemStack>]) -> bool {
    static INIT: Once = Once::new();
    INIT.call_once(|| candidate_policy::register().unwrap());
    let before = grid.to_vec();
    let input = Input::of(grid, 3).unwrap();
    let old = old_anvil::denies(&input, recipe) || old_melter::denies(&input, recipe);
    let new = matches!(pluto_matching::evaluate(&input, recipe), Outcome::Denied { .. });
    assert_eq!(new, old);
    assert_eq!(grid, before.as_slice());
    for (a, b) in grid.iter().zip(&before) {
        if let (Some(a), Some(b)) = (a,b) {
            assert_eq!(a.data().and_then(|d| d.wire.patch()), b.data().and_then(|d| d.wire.patch()));
        }
    }
    new
}
#[test]
fn unrelated_shapes_outputs_and_specials_keep_old_continuation() {
    let grid = grid(vec![fake(material())]);
    let rows = [shaped(ItemStack::new("example:other",1)),
        Recipe::Shapeless { ingredients: vec![], result: template(expected(material())) },
        Recipe::Special(Special::RepairItem)];
    for r in rows { assert!(!compare(&r, &grid)); }
}
#[test]
fn station_subtraction_requires_the_exact_shape_and_ingredient_sets() {
    let ing = |ids: &[&str]| Ingredient::of(ids.iter().map(|s| s.to_string()).collect());
    for (width, first, second, deny) in [
        (1, vec!["tconstruct:pattern"], vec!["tconstruct:crafting_station"], true),
        (2, vec!["tconstruct:pattern"], vec!["tconstruct:crafting_station"], false),
        (1, vec!["tconstruct:pattern", "example:other"], vec!["tconstruct:crafting_station"], false),
        (1, vec!["tconstruct:pattern"], vec!["minecraft:crafting_table"], false),
    ] {
        let r=Recipe::Shaped { width, height:2, cells:vec![Some(ing(&first)),Some(ing(&second))], result:template(ItemStack::new("tconstruct:crafting_station",1)) };
        let mut g=vec![None;9];g[4]=Some(ItemStack::new("tconstruct:crafting_station",1));g[7]=Some(ItemStack::new("tconstruct:pattern",1));
        assert_eq!(compare(&r,&g),deny);
        g[4]=Some(ItemStack::new("minecraft:crafting_table",1));assert!(!compare(&r,&g));
    }
}
#[test]
fn missing_or_wrong_expected_material_keeps_the_old_pass() {
    for value in [None,Some(Tag::Int(0)),Some(Tag::compound([] as [(&str,Tag);0]))] {
        let out=value.map(expected).unwrap_or_else(||ItemStack::new("tconstruct:tinkers_anvil",1));
        assert!(!compare(&shaped(out),&grid(vec![fake(material())])));
    }
}
#[test]
fn exactly_three_matching_materials_include_empty_and_utf16_strings() {
    for units in [vec![],vec![0xd800],vec![0xfffd],vec![0x61,0xd800,0x62]] {
        let text=Tag::String(JavaString(units));
        assert!(!compare(&shaped(expected(text.clone())),&grid(vec![fake(text);3])));
    }
    // Preserve native lossy conversion: lone surrogate and U+FFFD compare equal.
    assert!(!compare(&shaped(expected(Tag::String(JavaString(vec![0xd800])))),&grid(vec![fake(Tag::String(JavaString(vec![0xfffd])));3])));
}
#[test]
fn fake_count_is_exactly_three_not_component_count_or_total_slots() {
    for n in [1,2,4] { assert!(compare(&shaped(expected(material())),&grid(vec![fake(material());n]))); }
    let mut g=vec![None;9];for i in [1,4,7] {g[i]=Some(fake(material()));}g[8]=Some(ItemStack::new("example:unrelated",99));
    assert!(!compare(&shaped(expected(material())),&g));
}
#[test]
fn malformed_or_missing_input_material_denies() {
    for bad in [stack("tconstruct:fake_storage_block",None),fake(Tag::Int(0)),fake(Tag::compound([] as [(&str,Tag);0])),fake(Tag::String("example:copper".into()))] {
        assert!(compare(&shaped(expected(material())),&grid(vec![fake(material()),bad,fake(material())])));
    }
}
#[test]
fn retained_wire_keeps_stored_queries_available_without_semantic_capture() {
    for nested in [false,true] {
        let mut item=fake(Tag::String("example:copper".into()));
        let mut opaque=ItemStack::new("example:opaque",1);let decoded=opaque.clone();
        opaque.data_mut().wire=Wire::new(decoded,Arc::from(vec![0xa5_u8,0xff]));
        if nested { item.data_mut().bundle_contents.push(opaque); }
        else { let decoded=item.clone();item.data_mut().wire=Wire::new(decoded,Arc::from(vec![0xa5_u8,0xff])); }
        assert!(crate::pluto_component_view::ItemFacts::capture(&item).view().components.is_none());
        assert!(compare(&shaped(expected(material())),&grid(vec![fake(material()),item,fake(material())])));
    }
}
#[test]
fn duplicate_stored_components_keep_first_entry_selection() {
    let mut out=expected(material());out.data_mut().other.push((CUSTOM.into(),Tag::compound([("tconstruct:anvil",Tag::compound([("RequiredMaterial",Tag::String("wrong".into()))]))])));
    let mut input=fake(material());input.data_mut().other.push((CUSTOM.into(),Tag::compound([("Material",Tag::String("wrong".into()))])));
    assert!(!compare(&shaped(out),&grid(vec![input;3])));
}
#[test]
fn tank_presence_including_zero_empty_and_malformed_denies() {
    for tank in [Tag::End,Tag::Int(0),Tag::compound([] as [(&str,Tag);0]),Tag::compound([("amount",Tag::Int(0))]),Tag::compound([("amount",Tag::Int(1000))])] {
        for output in ["tconstruct:seared_melter","tconstruct:seared_fluid_cannon"] {
            let gauge=stack("tconstruct:seared_fuel_gauge",Some(Tag::compound([("tconstruct:tank",tank.clone())])));
            assert!(compare(&shaped(ItemStack::new(output,1)),&grid(vec![gauge])));
        }
    }
}
#[test]
fn absent_tank_and_malformed_parent_preserve_native_get_semantics() {
    for custom in [None,Some(Tag::Int(9)),Some(Tag::compound([] as [(&str,Tag);0]))] {
        let gauge=stack("tconstruct:seared_ingot_gauge",custom);
        assert!(!compare(&shaped(ItemStack::new("tconstruct:seared_melter",1)),&grid(vec![gauge])));
    }
}
#[test]
fn tank_policy_does_not_claim_other_inputs_or_outputs() {
    let tank=Tag::compound([("tconstruct:tank",Tag::Int(4))]);
    assert!(!compare(&shaped(ItemStack::new("tconstruct:seared_melter",1)),&grid(vec![stack("example:not_gauge",Some(tank.clone()))])));
    assert!(!compare(&shaped(ItemStack::new("example:not_melter",1)),&grid(vec![stack("tconstruct:seared_fuel_gauge",Some(tank))])));
}
#[test]
fn raw_template_patch_is_not_substituted_for_native_create() {
    let mut r=shaped(expected(material()));
    if let Recipe::Shaped{result,..}=&mut r { result.components=Some(Tag::compound([("!minecraft:custom_data",Tag::compound([] as [(&str,Tag);0]))])); }
    assert!(compare(&r,&grid(vec![fake(Tag::String("wrong".into()));3])));
}
