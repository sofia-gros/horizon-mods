use horizon_world::nbt::Tag;
use std::collections::HashMap;
#[path = "../../mixins/tconstruct/source-melter-state.rs"]
mod tconstruct_source_melter_state;
#[path = "../../mixins/tconstruct/source-melting-recipes.rs"]
mod tconstruct_source_melting_recipes;
#[path = "../../mixins/tconstruct/source-smeltery-tank.rs"]
mod tconstruct_source_smeltery_tank;
use tconstruct_source_melter_state::Tank;
use tconstruct_source_melting_recipes::{FluidFuel, MeltingRecipe};
type Pos = (i32, i32, i32);
struct Info {
    name: String,
}
struct Facts;
impl Facts {
    fn get(&self, id: u32) -> Info {
        Info {
            name: if id == 1 {
                "tconstruct:seared_fuel_tank"
            } else {
                "minecraft:air"
            }
            .into(),
        }
    }
}
struct Res;
impl Res {
    fn facts(&self) -> Facts {
        Facts
    }
}
struct Chunks {
    owners: HashMap<Pos, Pos>,
}
impl Chunks {
    fn __horizon_mixins_tconstruct_smeltery_owners(&self) -> &HashMap<Pos, Pos> {
        &self.owners
    }
}
struct World {
    chunks: Chunks,
    tanks: HashMap<Pos, Tag>,
    recipes: HashMap<String, Vec<u8>>,
}
impl World {
    fn block(&self, x: i32, y: i32, z: i32) -> u32 {
        u32::from(self.tanks.contains_key(&(x, y, z)))
    }
}
struct Game {
    world: World,
    res: Res,
}
struct State {
    phase: u8,
    fuel: u32,
    fuel_quality: u32,
    temperature: u32,
    rate: u32,
    tank: tconstruct_source_smeltery_tank::SmelteryTank,
    progress: i32,
}
impl State {
    fn can_heat_slot(&self, _: usize, temperature: u32, required: u32) -> bool {
        self.progress != -1 && temperature >= required
    }
}
mod store {
    use super::*;
    pub(super) fn read(w: &World, p: Pos, _: &str) -> Option<Tag> {
        w.tanks.get(&p).cloned()
    }
}
mod tconstruct_source_smeltery_fuel {
    include!(concat!(env!("SELECTED_FUEL_EXTRACT"), "/capacity.rs"));
}
mod tconstruct_source_melter_world {
    use super::*;
    pub(super) fn fluid_fuel(w: &World, id: &str, n: u32) -> Option<FluidFuel> {
        let r = FluidFuel::parse(w.recipes.get(id)?, &|c| c.is_empty())?;
        (r.fluid.amount <= n).then_some(r)
    }
    pub(super) fn alloy(
        _: &World,
        _: &[(String, u32)],
        _: &[String],
        _: u32,
        _: u32,
    ) -> Option<(String, Vec<(String, u32)>, String, u32)> {
        None
    }
}
mod runtime {
    use super::*;
    fn ensure(g: &mut Game, p: Pos, _: &str) -> bool {
        g.world.tanks.contains_key(&p)
    }
    include!(concat!(env!("SELECTED_FUEL_EXTRACT"), "/helpers.rs"));
    pub(super) fn select(
        game: &mut Game,
        state: &mut State,
        recipes: &[Option<(String, MeltingRecipe, String, Vec<(String, u32)>)>],
        foundry: bool,
    ) -> Option<(Pos, &'static str, Tag)> {
        let pos = (0, 1, 0);
        include!(concat!(env!("SELECTED_FUEL_EXTRACT"), "/selection.rs"))
    }
}
const COLD: Pos = (0, 0, 0);
const HOT: Pos = (1, 0, 0);
fn fixture() -> (
    Game,
    State,
    Vec<Option<(String, MeltingRecipe, String, Vec<(String, u32)>)>>,
) {
    let mut tanks = HashMap::new();
    for (p, id) in [(COLD, "minecraft:lava"), (HOT, "tconstruct:blazing_blood")] {
        let mut tank = Tank::new(4000);
        tank.insert(id, 1000);
        tanks.insert(p, tank.to_nbt());
    }
    let mut recipes = HashMap::new();
    for (id, file) in [
        ("minecraft:lava", "lava"),
        ("tconstruct:blazing_blood", "blaze"),
    ] {
        recipes.insert(
            id.into(),
            std::fs::read(
                std::path::Path::new(&std::env::var("SELECTED_FUEL_RECIPES").unwrap())
                    .join(format!("{file}.json")),
            )
            .unwrap(),
        );
    }
    let recipe=MeltingRecipe::parse(br#"{"type":"tconstruct:melting","ingredient":{"item":"test:metal"},"result":{"fluid":"test:molten","amount":90},"temperature":1250,"time":50}"#,&|_|true).unwrap();
    (
        Game {
            world: World {
                chunks: Chunks {
                    owners: HashMap::from([(HOT, (0, 1, 0)), (COLD, (0, 1, 0))]),
                },
                tanks,
                recipes,
            },
            res: Res,
        },
        State {
            phase: 0,
            fuel: 0,
            fuel_quality: 0,
            temperature: 0,
            rate: 0,
            tank: tconstruct_source_smeltery_tank::SmelteryTank::new(2160),
            progress: 0,
        },
        vec![Some((
            "test:metal".into(),
            recipe,
            "test:molten".into(),
            vec![],
        ))],
    )
}
#[cfg(test)]
mod selected_fuel_tests {
    use super::*;
    #[test]
    fn cold_selected_fuel_never_debits_hotter_later_tank() {
        for foundry in [false, true] {
            let (mut g, mut s, r) = fixture();
            let before = g.world.tanks.clone();
            assert!(runtime::select(&mut g, &mut s, &r, foundry).is_none());
            assert_eq!(s.fuel, 0);
            assert_eq!(g.world.tanks, before);
        }
    }
    #[test]
    fn missing_insufficient_or_zero_temperature_fuel_allows_fallback() {
        for mode in 0..3 {
            let (mut g, mut s, r) = fixture();
            match mode {
                0 => {
                    g.world.recipes.remove("minecraft:lava");
                }
                1 => {
                    let mut t = Tank::new(4000);
                    t.insert("minecraft:lava", 1);
                    g.world.tanks.insert(COLD, t.to_nbt());
                }
                _ => {
                    let mut v: serde_json::Value =
                        serde_json::from_slice(&g.world.recipes["minecraft:lava"]).unwrap();
                    v["temperature"] = 0.into();
                    g.world
                        .recipes
                        .insert("minecraft:lava".into(), serde_json::to_vec(&v).unwrap());
                }
            }
            let before = g.world.tanks.clone();
            let update = runtime::select(&mut g, &mut s, &r, false).unwrap();
            assert_eq!(update.0, HOT);
            assert!(s.fuel > 0);
            assert_eq!(s.temperature, 1500);
            assert_eq!(g.world.tanks, before);
        }
    }
    #[test]
    fn loaded_fuel_and_other_phases_do_not_select_again() {
        for (fuel, phase) in [(99, 0), (0, 1), (0, 2), (0, 3)] {
            let (mut g, mut s, r) = fixture();
            s.fuel = fuel;
            s.phase = phase;
            assert!(runtime::select(&mut g, &mut s, &r, false).is_none());
            assert_eq!(s.fuel, fuel);
        }
    }
    #[test]
    fn new_snapshot_after_reload_changes_selection_without_cached_temperature() {
        let (mut g, mut s, r) = fixture();
        assert!(runtime::select(&mut g, &mut s, &r, false).is_none());
        let mut v: serde_json::Value =
            serde_json::from_slice(&g.world.recipes["minecraft:lava"]).unwrap();
        v["temperature"] = 1300.into();
        g.world
            .recipes
            .insert("minecraft:lava".into(), serde_json::to_vec(&v).unwrap());
        let before = g.world.tanks.clone();
        let update = runtime::select(&mut g, &mut s, &r, false).unwrap();
        assert_eq!(update.0, COLD);
        assert_eq!(s.temperature, 1300);
        assert_eq!(Tank::from_nbt(&update.2, 4000).unwrap().amount, 950);
        assert_eq!(g.world.tanks, before);
    }
    #[test]
    fn selected_fuel_patch_preserves_extensions_and_is_only_staged() {
        let (mut g, mut s, mut r) = fixture();
        r[0].as_mut().unwrap().1.temperature = 500;
        if let Tag::Compound(f) = g.world.tanks.get_mut(&COLD).unwrap() {
            f.insert("future".into(), Tag::Int(73));
        }
        let before = g.world.tanks.clone();
        let update = runtime::select(&mut g, &mut s, &r, false).unwrap();
        assert_eq!(update.2.get("future"), Some(&Tag::Int(73)));
        assert_eq!(g.world.tanks, before);
    }
    #[test]
    fn unsupported_tagged_or_malformed_fuel_stays_untouched_and_allows_fallback() {
        for metadata in [Tag::compound([("test", Tag::Int(9))]), Tag::Int(9)] {
            let (mut g, mut s, r) = fixture();
            if let Tag::Compound(f) = g.world.tanks.get_mut(&COLD).unwrap() {
                f.insert("fluid_tag".into(), metadata);
            }
            let before = g.world.tanks.clone();
            let update = runtime::select(&mut g, &mut s, &r, false).unwrap();
            assert_eq!(update.0, HOT);
            assert_eq!(g.world.tanks, before);
        }
    }
}
