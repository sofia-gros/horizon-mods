//! Adapter fixture for the actual guard body, not a native host integration test.
#![allow(dead_code)]
extern crate self as horizon_world;
use std::collections::{BTreeMap, HashSet};

pub mod nbt {
    use super::BTreeMap;
    #[derive(Clone)]
    pub struct Text(pub String);
    impl Text {
        pub fn to_string_lossy(&self) -> String { self.0.clone() }
    }
    #[derive(Clone)]
    pub enum Tag { String(Text), Compound(BTreeMap<String, Tag>) }
    impl Tag {
        pub fn get(&self, name: &str) -> Option<&Self> {
            match self { Self::Compound(map) => map.get(name), _ => None }
        }
    }
}
use nbt::{Tag, Text};
#[derive(Clone)]
struct Data(BTreeMap<String, Tag>);
impl Data { fn other(&self, name: &str) -> Option<&Tag> { self.0.get(name) } }
#[derive(Clone)]
struct ItemStack { item: String, data: Option<Data> }
impl ItemStack { fn data(&self) -> Option<&Data> { self.data.as_ref() } }
struct Input<'a>(&'a [ItemStack]);
impl<'a> Input<'a> { fn stacks(&self) -> Vec<&'a ItemStack> { self.0.iter().collect() } }
struct Ingredient { items: HashSet<String> }
struct Template(ItemStack);
impl Template { fn create(&self) -> ItemStack { self.0.clone() } }
enum Recipe { Shaped { width: usize, height: usize, cells: Vec<Option<Ingredient>>, result: Template }, Other }

include!("../../../mixins/tconstruct/anvil-material-recipe.rs");

#[cfg(test)]
mod tests {
    use super::*;
    fn text(value: &str) -> Tag { Tag::String(Text(value.into())) }
    fn compound(key: &str, value: Tag) -> Tag { Tag::Compound(BTreeMap::from([(key.into(), value)])) }
    fn stack(item: &str, material: Option<&str>, output: bool) -> ItemStack {
        let data = material.map(|value| {
            let tag = if output { compound("tconstruct:anvil", compound("RequiredMaterial", text(value))) }
                      else { compound("Material", text(value)) };
            Data(BTreeMap::from([("minecraft:custom_data".into(), tag)]))
        });
        ItemStack { item: item.into(), data }
    }
    fn ingredient(item: &str) -> Option<Ingredient> {
        Some(Ingredient { items: [item.into()].into_iter().collect() })
    }
    fn recipe(forge: bool) -> Recipe {
        let f = "tconstruct:fake_storage_block";
        let s = "tconstruct:seared_bricks";
        let cells = if forge { vec![ingredient(s), ingredient(s), ingredient(s), ingredient(f),
            ingredient("tconstruct:tinker_station"), ingredient(f), ingredient(f), None, ingredient(f)] }
        else { vec![ingredient(f), ingredient(f), ingredient(f), None, ingredient(s), None,
            ingredient(s), ingredient(s), ingredient(s)] };
        Recipe::Shaped { width: 3, height: 3, cells,
            result: Template(stack("tconstruct:tinkers_anvil", Some("tconstruct:bronze"), true)) }
    }
    fn input(count: usize) -> Vec<ItemStack> {
        (0..count).map(|_| stack("tconstruct:fake_storage_block", Some("tconstruct:bronze"), false)).collect()
    }
    #[test]
    fn three_block_anvil_and_four_block_forge_are_distinct() {
        for (forge, count) in [(false, 3), (true, 4)] {
            assert!(tconstruct_anvil_material_recipe(&Input(&input(count)), &recipe(forge)).is_continue());
            assert!(tconstruct_anvil_material_recipe(&Input(&input(7 - count)), &recipe(forge)).is_break());
        }
    }
    #[test]
    fn every_material_must_match_and_missing_data_is_rejected() {
        for forge in [false, true] {
            let count = if forge { 4 } else { 3 };
            for bad in 0..count {
                for material in [None, Some("tconstruct:electrum")] {
                    let mut stacks = input(count);
                    stacks[bad] = stack("tconstruct:fake_storage_block", material, false);
                    assert!(tconstruct_anvil_material_recipe(&Input(&stacks), &recipe(forge)).is_break());
                }
            }
        }
    }
    #[test]
    fn arbitrary_four_block_shapes_do_not_get_the_forge_exception() {
        for index in 0..9 {
            let Recipe::Shaped { width, height, mut cells, result } = recipe(true) else { unreachable!() };
            cells[index] = if index == 7 { ingredient("minecraft:stone") } else { None };
            let changed = Recipe::Shaped { width, height, cells, result };
            assert!(tconstruct_anvil_material_recipe(&Input(&input(4)), &changed).is_break());
        }
        let Recipe::Shaped { height, cells, result, .. } = recipe(true) else { unreachable!() };
        assert!(tconstruct_anvil_material_recipe(&Input(&input(4)),
            &Recipe::Shaped { width: 2, height, cells, result }).is_break());
    }
    #[test]
    fn unrelated_and_ordinary_outputs_remain_native() {
        assert!(tconstruct_anvil_material_recipe(&Input(&[]), &Recipe::Other).is_continue());
        let Recipe::Shaped { width, height, cells, .. } = recipe(true) else { unreachable!() };
        let ordinary = Recipe::Shaped { width, height, cells,
            result: Template(stack("tconstruct:tinkers_anvil", None, true)) };
        assert!(tconstruct_anvil_material_recipe(&Input(&[]), &ordinary).is_continue());
    }
}
