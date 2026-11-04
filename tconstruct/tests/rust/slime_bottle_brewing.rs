use std::collections::{HashMap, HashSet};
#[derive(Clone, Debug)] struct ItemStack { item: Box<str>, count: u32 }
impl ItemStack { fn new(id: &str, count: u32) -> Self { Self { item: id.into(), count } } }
struct Items { ids: HashSet<String>, item_tags: HashMap<String, HashSet<String>> }
impl Items { fn get(&self, id: &str) -> Option<()> { self.ids.contains(id).then_some(()) } }
mod brewing {
    use super::*;
    #[derive(Clone)] pub struct PotionIngredient { pub items: HashSet<String>, pub potions: Option<HashSet<String>> }
    pub struct BrewingRecipe { pub id: String, pub input: PotionIngredient, pub reagent: PotionIngredient, pub output: ItemStack }
    pub struct Brewing { pub recipes: Vec<BrewingRecipe>, pub reagents: HashSet<String>, pub inputs: HashSet<String> }
}
include!("../../mixins/tconstruct/source-splash-bottle-brewing.rs");
fn empty() -> brewing::Brewing { brewing::Brewing { recipes: vec![], reagents: HashSet::new(), inputs: HashSet::new() } }
fn items() -> Items { Items { ids: ["minecraft:glass_bottle", "minecraft:magma_block", "tconstruct:magma_bottle", "tconstruct:earth_congealed_slime", "tconstruct:earth_slime_bottle", "tconstruct:sky_congealed_slime", "tconstruct:sky_slime_bottle", "tconstruct:ender_congealed_slime", "tconstruct:ender_slime_bottle", "tconstruct:ichor_congealed_slime", "tconstruct:ichor_slime_bottle"].into_iter().map(str::to_owned).collect(), item_tags: HashMap::new() } }
#[test] fn adds_only_registered_source_recipes() {
    let v = tconstruct_splash_bottle_brewing(empty(), &items());
    assert_eq!(v.recipes.len(), 5);
    for (i, o) in [("tconstruct:earth_congealed_slime", "tconstruct:earth_slime_bottle"), ("tconstruct:sky_congealed_slime", "tconstruct:sky_slime_bottle"), ("tconstruct:ender_congealed_slime", "tconstruct:ender_slime_bottle"), ("minecraft:magma_block", "tconstruct:magma_bottle"), ("tconstruct:ichor_congealed_slime", "tconstruct:ichor_slime_bottle")] {
        let r = v.recipes.iter().find(|r| r.reagent.items.contains(i)).unwrap();
        assert!(r.input.items.contains("minecraft:glass_bottle")); assert_eq!(&*r.output.item, o); assert_eq!(r.output.count, 1); assert!(v.reagents.contains(i));
    }
    assert!(v.inputs.contains("minecraft:glass_bottle"));
}
#[test] fn missing_items_skip_without_partial_registration() {
    let mut i = items(); i.ids.remove("tconstruct:sky_slime_bottle"); i.ids.remove("minecraft:magma_block");
    let v = tconstruct_splash_bottle_brewing(empty(), &i); assert_eq!(v.recipes.len(), 3); assert!(!v.reagents.contains("minecraft:magma_block"));
    i.ids.remove("minecraft:glass_bottle"); assert!(tconstruct_splash_bottle_brewing(empty(), &i).recipes.is_empty());
}
#[test] fn existing_overlap_wins_and_repeated_application_is_idempotent() {
    let v = tconstruct_splash_bottle_brewing(empty(), &items()); let mut v = tconstruct_splash_bottle_brewing(v, &items()); assert_eq!(v.recipes.len(), 5);
    v.recipes[0].id = "pack:choice".into(); v.recipes[0].output = ItemStack::new("pack:output", 1);
    let v = tconstruct_splash_bottle_brewing(v, &items()); assert_eq!(v.recipes.len(), 5); assert_eq!(&*v.recipes[0].output.item, "pack:output");
}
