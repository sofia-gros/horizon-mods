//! Actual production tick body; Game, recipe snapshot wiring, provider and store are bounded adapters.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use std::collections::{BTreeMap, HashMap, HashSet};
#[path = "../../mixins/tconstruct/source-melter-state.rs"]
mod tconstruct_source_melter_state;
#[path = "../../mixins/tconstruct/source-melting-lookup.rs"]
mod tconstruct_source_melting_lookup;
#[path = "../../mixins/tconstruct/source-melting-recipes.rs"]
mod tconstruct_source_melting_recipes;
#[path = "../../mixins/tconstruct/source-tags.rs"]
mod tconstruct_source_tags;
use tconstruct_source_melter_state::{Melter, Tank};
use tconstruct_source_melting_recipes::{FluidFuel, FoundryRecipe, MeltingRecipe};
mod tconstruct_source_fluid_container {
    pub(super) fn filled_bucket(_: &str) -> Option<String> {
        panic!("bucket casting is outside this fixture")
    }
}
mod tconstruct_source_smeltery_fuel {
    include!(concat!(env!("MELTER_RUNTIME_EXTRACT"), "/capacity.rs"));
}
mod tconstruct_source_melter_render {
    pub(super) fn encode_heat(_: Option<(u32, u32)>, _: u32, _: i32) -> i32 {
        0
    }
}
type Pos = (i32, i32, i32);
const POS: Pos = (0, 1, 0);
const BELOW: Pos = (0, 0, 0);
#[derive(Clone, Debug, PartialEq)]
enum BlockEntity {
    Chest(Vec<Option<ItemStack>>),
    Other,
}
struct Info {
    name: String,
}
struct Facts;
impl Facts {
    fn get(&self, id: u32) -> Info {
        Info {
            name: match id {
                1 => "tconstruct:seared_heater",
                2 => "tconstruct:seared_fuel_tank",
                _ => "minecraft:air",
            }
            .into(),
        }
    }
}
struct ItemInfo {
    crafting_remainder: Option<String>,
    max_stack: u32,
}
struct Providers;
impl Providers {
    fn burn_time(&self, _: &HashMap<std::sync::Arc<str>, ItemInfo>, s: &ItemStack, _: bool) -> i32 {
        match &*s.item {
            "minecraft:coal" => 1600,
            "minecraft:lava_bucket" => 20000,
            "test:tiny_fuel" => 3,
            _ => 0,
        }
    }
}
struct Res {
    items: HashMap<std::sync::Arc<str>, ItemInfo>,
}
impl Res {
    fn facts(&self) -> Facts {
        Facts
    }
    fn providers(&self) -> Providers {
        Providers
    }
    fn items(&self) -> &HashMap<std::sync::Arc<str>, ItemInfo> {
        &self.items
    }
}
struct Chunks {
    recipes: Option<BTreeMap<String, Vec<u8>>>,
}
impl Chunks {
    fn __horizon_mixins_tconstruct_recipe_snapshot(&self) -> &Option<BTreeMap<String, Vec<u8>>> {
        &self.recipes
    }
}
struct World {
    chunks: Chunks,
    below: u32,
    block_entities: HashMap<Pos, BlockEntity>,
    saved: HashMap<(Pos, String), Tag>,
    modified: HashSet<(i32, i32)>,
    reject: bool,
    writes: usize,
}
impl World {
    fn block(&self, _: i32, y: i32, _: i32) -> u32 {
        if y == 0 {
            self.below
        } else {
            0
        }
    }
}
struct Game<'a> {
    world: World,
    res: Res,
    drops: Vec<ItemStack>,
    lit: Vec<(Pos, bool)>,
    _life: std::marker::PhantomData<&'a ()>,
}
mod store {
    use super::*;
    pub(super) fn read(w: &World, p: Pos, k: &str) -> Option<Tag> {
        w.saved.get(&(p, k.into())).cloned()
    }
    pub(super) fn write_many(w: &mut World, updates: &[(Pos, &str, Tag)]) -> bool {
        if w.reject {
            return false;
        }
        for (p, k, _) in updates {
            if !w.saved.contains_key(&(*p, (*k).into())) {
                return false;
            }
        }
        for (p, k, v) in updates {
            w.saved.insert((*p, (*k).into()), v.clone());
        }
        w.writes += 1;
        true
    }
}
mod spill {
    pub(super) mod tconstruct_source_smeltery_spill {
        pub(crate) fn spill(
            g: &mut super::super::Game<'_>,
            _: super::super::Pos,
            v: Vec<super::super::ItemStack>,
        ) {
            g.drops.extend(v)
        }
    }
}
mod tconstruct_source_melter_world {
    use super::*;
    fn catalog<R>(
        w: &World,
        f: impl FnOnce(&tconstruct_source_melting_lookup::Catalog<'_>) -> R,
    ) -> R {
        let tags = HashMap::from([(
            "forge:ingots/copper".into(),
            HashSet::from(["minecraft:copper_ingot".into()]),
        )]);
        let mut fluids = HashMap::from([(
            "minecraft:lava".into(),
            HashSet::from(["minecraft:lava".into()]),
        )]);
        fluids.insert(
            "forge:molten_copper".into(),
            HashSet::from(["tconstruct:molten_copper".into()]),
        );
        let items = HashSet::from(["minecraft:copper_ingot".into()]);
        let known = HashSet::from(["minecraft:lava".into(), "tconstruct:molten_copper".into()]);
        f(&tconstruct_source_melting_lookup::Catalog {
            recipes: w.chunks.recipes.as_ref().unwrap(),
            item_tags: &tags,
            fluid_tags: &fluids,
            known_items: &items,
            known_fluids: &known,
        })
    }
    pub(super) fn melting_stack(
        w: &World,
        s: &ItemStack,
        no: Option<bool>,
    ) -> Result<Option<(String, MeltingRecipe, String)>, ()> {
        catalog(w, |c| {
            c.melting_with_rates(&s.item, no, &|v| v.is_empty(), (None, None))
        })
    }
    pub(super) fn fluid_fuel(w: &World, id: &str, n: u32) -> Option<FluidFuel> {
        catalog(w, |c| c.fluid_fuel(id, n, &|v| v.is_empty()))
    }
    pub(super) fn material_melting(
        _: &World,
        _: &ItemStack,
    ) -> Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> {
        None
    }
}
mod runtime {
    use super::*;
    fn no_crafting_remainder(_: &Game<'_>, _: &ItemStack) -> Option<bool> {
        Some(true)
    }
    fn damaged_recipe(
        _: &Game<'_>,
        _: &ItemStack,
    ) -> Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> {
        None
    }
    fn lit(g: &mut Game<'_>, p: Pos, b: bool) {
        g.lit.push((p, b));
    }
    include!(concat!(env!("MELTER_RUNTIME_EXTRACT"), "/runtime.rs"));
}
fn fixture(below: u32, fuel: Option<ItemStack>) -> Game<'static> {
    let mut recipes = BTreeMap::new();
    for entry in std::fs::read_dir(std::env::var("SURVIVAL_RECIPES").unwrap()).unwrap() {
        let path = entry.unwrap().path();
        let name = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .trim_end_matches(".json")
            .replace("__", "/");
        recipes.insert(format!("tconstruct:{name}"), std::fs::read(path).unwrap());
    }
    let mut tank = Tank::new(4000);
    tank.insert("minecraft:lava", 1000);
    let mut g = Game {
        world: World {
            chunks: Chunks {
                recipes: Some(recipes),
            },
            below,
            block_entities: HashMap::from([
                (
                    POS,
                    BlockEntity::Chest(vec![
                        Some(ItemStack::new("minecraft:copper_ingot", 4)),
                        None,
                        None,
                    ]),
                ),
                (BELOW, BlockEntity::Chest(vec![fuel])),
            ]),
            saved: HashMap::from([
                ((POS, "tconstruct:melter".into()), Melter::new().to_nbt()),
                ((BELOW, "tconstruct:fuel_tank".into()), tank.to_nbt()),
            ]),
            modified: HashSet::new(),
            reject: false,
            writes: 0,
        },
        res: Res {
            items: HashMap::new(),
        },
        drops: vec![],
        lit: vec![],
        _life: std::marker::PhantomData,
    };
    for id in [
        "minecraft:coal",
        "minecraft:bucket",
        "minecraft:copper_ingot",
        "test:tiny_fuel",
    ] {
        g.res.items.insert(
            id.into(),
            ItemInfo {
                crafting_remainder: None,
                max_stack: 64,
            },
        );
    }
    g.res.items.insert(
        "minecraft:lava_bucket".into(),
        ItemInfo {
            crafting_remainder: Some("minecraft:bucket".into()),
            max_stack: 1,
        },
    );
    g
}
fn state(g: &Game<'_>) -> Melter {
    Melter::from_nbt(&store::read(&g.world, POS, "tconstruct:melter").unwrap()).unwrap()
}
fn tank(g: &Game<'_>) -> Tank {
    Tank::from_nbt(
        &store::read(&g.world, BELOW, "tconstruct:fuel_tank").unwrap(),
        4000,
    )
    .unwrap()
}
#[cfg(test)]
mod melter_fuel_tests {
    use super::*;
    #[test]
    fn liquid_driver_melts_controller_copper_with_exact_phase_budget() {
        let mut g = fixture(2, None);
        let mut ticks = 0;
        while state(&g).tank.amount < 360 && ticks < 2000 {
            runtime::tick_melter(&mut g, POS);
            ticks += 1;
        }
        let m = state(&g);
        assert_eq!(m.tank.amount, 360);
        assert!(m.slots.iter().all(Option::is_none));
        // Four items: source copper recipe time*10/rate heating steps, plus one completion step each.
        let recipe = tconstruct_source_melter_world::melting_stack(
            &g.world,
            &ItemStack::new("minecraft:copper_ingot", 1),
            Some(true),
        )
        .unwrap()
        .unwrap()
        .1;
        let steps = 4 * ((recipe.time * 10).div_ceil(10) + 1);
        assert_eq!(ticks, steps * 4 - 1);
        let loads = steps.div_ceil(100);
        assert_eq!(tank(&g).amount, 1000 - 50 * loads);
        assert_eq!(m.fuel, 100 * loads - steps);
        assert_eq!(m.phase, 3);
        assert_eq!(g.world.writes, ticks as usize);
        assert!(g.drops.is_empty());
    }
    #[test]
    fn heater_consumes_one_fuel_and_publishes_bucket_only_after_commit() {
        let mut g = fixture(1, Some(ItemStack::new("minecraft:lava_bucket", 1)));
        let before = g.world.saved.clone();
        let inv = g.world.block_entities.clone();
        g.world.reject = true;
        runtime::tick_melter(&mut g, POS);
        assert_eq!(g.world.saved, before);
        assert_eq!(g.world.block_entities, inv);
        assert!(g.drops.is_empty());
        assert!(g.lit.is_empty());
        assert!(g.world.modified.is_empty());
        g.world.reject = false;
        runtime::tick_melter(&mut g, POS);
        assert_eq!(state(&g).fuel, 5000);
        assert_eq!(g.drops, vec![ItemStack::new("minecraft:bucket", 1)]);
        assert_eq!(
            g.world.block_entities[&BELOW],
            BlockEntity::Chest(vec![None])
        );
        runtime::tick_melter(&mut g, POS);
        assert_eq!(state(&g).fuel, 5000);
        runtime::tick_melter(&mut g, POS);
        assert_eq!(state(&g).fuel, 4999);
        assert_eq!(g.drops.len(), 1);
    }
    #[test]
    fn liquid_failure_is_atomic_and_completed_full_output_never_refuels() {
        let mut g = fixture(2, None);
        g.world.reject = true;
        let before = g.world.saved.clone();
        let inv = g.world.block_entities.clone();
        runtime::tick_melter(&mut g, POS);
        assert_eq!(g.world.saved, before);
        assert_eq!(g.world.block_entities, inv);
        g.world.reject = false;
        let mut m = state(&g);
        m.slots[0] = Some(ItemStack::new("minecraft:copper_ingot", 4));
        m.progress[0] = -1;
        m.tank.insert("tconstruct:molten_copper", 2160);
        g.world
            .saved
            .insert((POS, "tconstruct:melter".into()), m.to_nbt());
        for _ in 0..8 {
            runtime::tick_melter(&mut g, POS);
        }
        assert_eq!(tank(&g).amount, 1000);
        assert_eq!(state(&g).fuel, 0);
        assert_eq!(state(&g).slots[0].as_ref().unwrap().count, 4);
    }
    #[test]
    fn no_recipe_low_temperature_and_short_burn_do_not_consume() {
        for fuel in ["minecraft:coal", "test:tiny_fuel"] {
            let mut g = fixture(1, Some(ItemStack::new(fuel, 2)));
            if fuel == "minecraft:coal" {
                g.world.block_entities.insert(
                    POS,
                    BlockEntity::Chest(vec![
                        Some(ItemStack::new("minecraft:stone", 1)),
                        None,
                        None,
                    ]),
                );
            }
            runtime::tick_melter(&mut g, POS);
            assert_eq!(state(&g).fuel, 0);
            assert_eq!(
                g.world.block_entities[&BELOW],
                BlockEntity::Chest(vec![Some(ItemStack::new(fuel, 2))])
            );
        }
        let mut g = fixture(2, None);
        for bytes in g.world.chunks.recipes.as_mut().unwrap().values_mut() {
            let mut v: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            if v["type"] == "tconstruct:melting_fuel" {
                v["temperature"] = 0.into();
                *bytes = serde_json::to_vec(&v).unwrap();
            }
        }
        runtime::tick_melter(&mut g, POS);
        assert_eq!(tank(&g).amount, 1000);
        assert_eq!(state(&g).fuel, 0);
    }
    #[test]
    fn coal_driver_melts_without_draining_liquid_tank() {
        let mut g = fixture(1, Some(ItemStack::new("minecraft:coal", 2)));
        let mut ticks = 0;
        while state(&g).tank.amount < 360 && ticks < 4000 {
            runtime::tick_melter(&mut g, POS);
            ticks += 1;
        }
        assert_eq!(state(&g).tank.amount, 360);
        assert_eq!(tank(&g).amount, 1000);
        assert_eq!(
            g.world.block_entities[&BELOW],
            BlockEntity::Chest(vec![Some(ItemStack::new("minecraft:coal", 1))])
        );
        assert_eq!(state(&g).fuel, 400 - (ticks + 1) / 4);
        assert!(g.drops.is_empty());
    }
    #[test]
    fn rejected_completion_preserves_live_item_and_pending_output() {
        let mut g = fixture(2, None);
        let mut m = Melter::new();
        m.slots[0] = Some(ItemStack::new("minecraft:copper_ingot", 4));
        m.progress[0] = -1;
        m.phase = 2;
        g.world
            .saved
            .insert((POS, "tconstruct:melter".into()), m.to_nbt());
        let before = g.world.saved.clone();
        let inv = g.world.block_entities.clone();
        g.world.reject = true;
        runtime::tick_melter(&mut g, POS);
        assert_eq!(g.world.saved, before);
        assert_eq!(g.world.block_entities, inv);
        assert!(g.lit.is_empty());
        g.world.reject = false;
        runtime::tick_melter(&mut g, POS);
        assert_eq!(state(&g).tank.amount, 90);
        assert_eq!(state(&g).slots[0].as_ref().unwrap().count, 3);
        assert_eq!(tank(&g).amount, 1000);
    }
    #[test]
    fn missing_structure_burns_stored_fuel_without_heating() {
        let mut g = fixture(2, None);
        runtime::tick_melter(&mut g, POS);
        g.world.below = 0;
        runtime::tick_melter(&mut g, POS);
        runtime::tick_melter(&mut g, POS);
        assert_eq!(state(&g).fuel, 99);
        assert_eq!(state(&g).progress, [0; 3]);
        assert!(g.lit.is_empty());
    }
}
