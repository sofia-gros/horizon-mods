use super::*;
use horizon_items::{items::ItemStack, item_nbt::{item_from_nbt, item_to_nbt}};
use std::collections::{BTreeMap, HashMap, HashSet};
use tconstruct_source_melter_state::{Casting, Melter, Tank};
use tconstruct_source_smeltery_tank::{FluidAmount, SmelteryTank};

struct Fixture {
    recipes: BTreeMap<String, Vec<u8>>,
    items: HashSet<String>, fluids: HashSet<String>,
    item_tags: HashMap<String, HashSet<String>>, fluid_tags: HashMap<String, HashSet<String>>,
}
impl Fixture {
    fn new() -> Self {
        let dir = std::env::var("SURVIVAL_RECIPES").expect("use run_survival_chain_rust.sh");
        let recipes = std::fs::read_dir(dir).unwrap().map(|entry| {
            let path = entry.unwrap().path();
            (path.file_name().unwrap().to_str().unwrap().replace("__", "/"), std::fs::read(path).unwrap())
        }).collect();
        // Explicit minimal resolved registry snapshot, not world registry loading.
        let item_tags = [("forge:ingots/copper", "minecraft:copper_ingot"),
            ("forge:ingots/gold", "minecraft:gold_ingot"),
            ("forge:storage_blocks/rose_gold", "tconstruct:rose_gold_block"),
            ("tconstruct:smeltery_bricks", "tconstruct:seared_bricks")]
            .map(|(tag, id)| (tag.into(), HashSet::from([id.into()]))).into_iter().collect();
        let fluid_tags = ["copper", "gold", "rose_gold"].map(|metal|
            (format!("forge:molten_{metal}"), HashSet::from([format!("tconstruct:molten_{metal}")]))).into_iter().collect();
        Self { recipes, item_tags, fluid_tags,
            items: ["minecraft:copper_ingot", "minecraft:gold_ingot", "tconstruct:rose_gold_block",
                "tconstruct:seared_bricks", "tconstruct:smeltery_controller"].map(str::to_owned).into_iter().collect(),
            fluids: ["tconstruct:molten_copper", "tconstruct:molten_gold", "tconstruct:molten_rose_gold", "minecraft:lava"].map(str::to_owned).into_iter().collect() }
    }
    fn catalog(&self) -> tconstruct_source_melting_lookup::Catalog<'_> {
        tconstruct_source_melting_lookup::Catalog { recipes: &self.recipes, item_tags: &self.item_tags,
            fluid_tags: &self.fluid_tags, known_items: &self.items, known_fluids: &self.fluids }
    }
    fn melt(&self, item: &str, count: u32) -> Tank {
        let (_, recipe, fluid) = self.catalog().melting(item, &|_| true).unwrap();
        let mut m = Melter::new();
        m.slots[0] = Some(ItemStack::new(item, count));
        // Heat scheduling is supplied by this fixture, not a world tick.
        m.temperature = 1000; m.rate = 10; m.fuel = 77; m.fuel_quality = 100;
        for _ in 0..1000 {
            if m.slots[0].is_none() { break; }
            m.heat_slot(0, recipe.temperature, recipe.time, &fluid, recipe.output.amount);
            let saved = m.to_nbt();
            m = Melter::from_nbt(&saved).unwrap();
            assert_eq!(m.to_nbt(), saved);
        }
        assert!(m.slots[0].is_none());
        assert_eq!(m.tank.amount, count * recipe.output.amount);
        // heat_slot does not debit fuel. Do not claim to test world fuel use.
        assert_eq!(m.fuel, 77);
        m.tank
    }
    fn cast(&self, fluid: &str, input: Option<&str>) -> (Casting, u32) {
        let (id, recipe, output) = self.catalog().casting(true, input, fluid, &|_| true).unwrap();
        let mut c = Casting::new();
        c.input = input.map(|id| ItemStack::new(id, 1));
        assert!(c.prepare(&id, recipe.fluid.amount, recipe.cooling_time,
            ItemStack::new(&output, 1), recipe.consumes_cast, recipe.switch_slots));
        (c, recipe.fluid.amount)
    }
}
fn finish(mut c: Casting) -> ItemStack {
    for _ in 0..1000 {
        let done = c.tick(true);
        let saved = c.to_nbt(); c = Casting::from_nbt(&saved).unwrap();
        assert_eq!(c.to_nbt(), saved);
        if done {
            assert_eq!(c.tank.amount, 0);
            assert!(c.input.is_none());
            return c.output.take().unwrap();
        }
    }
    panic!("casting did not finish")
}
fn amount(id: &str, n: u32) -> FluidAmount { FluidAmount { id: id.into(), tag: None, amount: n } }

#[test]
fn copper_controller_and_rose_gold_chain_retain_quantities() {
    let f = Fixture::new();
    let mut copper = f.melt("minecraft:copper_ingot", 4);
    let (mut controller, needed) = f.cast("tconstruct:molten_copper", Some("tconstruct:seared_bricks"));
    assert_eq!(needed, 360);
    assert!(controller.tank.insert(&copper.take(needed).unwrap(), needed));
    assert_eq!(copper.amount, 0);
    assert_eq!(finish(controller).item.as_ref(), "tconstruct:smeltery_controller");

    let lava = tconstruct_source_melting_recipes::FluidFuel::parse(
        &f.recipes["smeltery/melting/fuel/lava.json"], &|_| true).unwrap();
    assert_eq!((lava.fluid.amount, lava.duration, lava.temperature, lava.rate), (50, 100, 1000, 10));
    let mut fuel = Tank::new(4000); assert!(fuel.insert("minecraft:lava", 1000));
    assert_eq!(fuel.take(lava.fluid.amount).as_deref(), Some("minecraft:lava"));
    fuel = Tank::from_nbt(&fuel.to_nbt(), 4000).unwrap(); assert_eq!(fuel.amount, 950);
    // One source fuel unit only; this is not a predicted whole-route fuel budget.

    let mut copper = f.melt("minecraft:copper_ingot", 14);
    let mut gold = f.melt("minecraft:gold_ingot", 14);
    let mut tank = SmelteryTank::new(2160);
    let mut blocks = Vec::new();
    for _ in 0..14 {
        for input in [&mut copper, &mut gold] {
            assert!(tank.insert(&input.take(90).unwrap(), 90));
        }
        let available: Vec<_> = tank.fluids.iter().map(|f| (f.id.clone(), f.amount)).collect();
        let (_, inputs, output, n) = f.catalog().alloy(&available, tank.capacity, lava.temperature, &|_| true).unwrap();
        assert!(tank.alloy(&inputs.iter().map(|(id,n)| amount(id,*n)).collect::<Vec<_>>(), &amount(&output,n)));
        tank = SmelteryTank::from_nbt(&tank.to_nbt()).unwrap();
        if tank.amount("tconstruct:molten_rose_gold") >= 810 {
            let (mut c, needed) = f.cast("tconstruct:molten_rose_gold", None);
            assert_eq!(needed, 810); assert!(tank.take("tconstruct:molten_rose_gold", needed));
            assert!(c.tank.insert("tconstruct:molten_rose_gold", needed)); blocks.push(finish(c));
        }
    }
    assert_eq!((copper.amount, gold.amount, tank.total(), blocks.len()), (0,0,90,3));
    assert!(blocks.iter().all(|b| b.item.as_ref() == "tconstruct:rose_gold_block" && b.count == 1));
    // Verify the shipped native recipe cost, not a fabricated crafting transaction.
    let recipe: serde_json::Value = serde_json::from_slice(&std::fs::read(
        std::path::Path::new(&std::env::var("ROOT").unwrap()).join("tconstruct/content/data/tconstruct/recipe/tinkers_anvil_rose_gold.json")).unwrap()).unwrap();
    let pattern: String = recipe["pattern"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
    assert_eq!(pattern.matches('m').count(), blocks.len()); assert_eq!(pattern.matches('s').count(), 4);
    assert_eq!(recipe["key"]["m"], "tconstruct:rose_gold_block");
    assert_eq!(recipe["key"]["s"], "#tconstruct:anvil_seared_blocks");
    assert_eq!(recipe["result"]["id"], "tconstruct:tinkers_anvil");
    for block in blocks { let restored = item_from_nbt(&item_to_nbt(&block)).unwrap();
        assert_eq!(restored.count, block.count); assert!(restored.same_item_same_components(&block)); }
}

#[test]
fn rejected_heat_alloy_and_cast_do_not_consume() {
    let f = Fixture::new();
    let (_, recipe, fluid) = f.catalog().melting("minecraft:copper_ingot", &|_| true).unwrap();
    let mut m = Melter::new(); m.slots[0] = Some(ItemStack::new("minecraft:copper_ingot",1));
    m.temperature = recipe.temperature - 1; m.rate = 10;
    let before = m.to_nbt(); assert!(!m.heat_slot(0,recipe.temperature,recipe.time,&fluid,90)); assert_eq!(m.to_nbt(),before);
    m.temperature = recipe.temperature; m.progress[0] = recipe.time as i32;
    assert!(m.tank.insert(&fluid, m.tank.capacity));
    assert!(!m.heat_slot(0,recipe.temperature,recipe.time,&fluid,90));
    assert_eq!(m.slots[0].as_ref().unwrap().count,1); assert_eq!(m.progress[0],-1);
    assert_eq!(m.tank.take(90).as_deref(),Some(fluid.as_str()));
    assert!(m.heat_slot(0,recipe.temperature,recipe.time,&fluid,90)); assert!(m.slots[0].is_none());
    let available = vec![("tconstruct:molten_copper".into(),90),("tconstruct:molten_gold".into(),90)];
    assert!(f.catalog().alloy(&available,2160,549,&|_| true).is_none());
    let mut tank = SmelteryTank::new(2160); assert!(tank.insert("tconstruct:molten_copper",90));
    let before = tank.to_nbt();
    assert!(!tank.alloy(&[amount("tconstruct:molten_copper",90), amount("tconstruct:molten_gold",90)],&amount("tconstruct:molten_rose_gold",180)));
    assert_eq!(tank.to_nbt(),before);
    let (mut c,needed) = f.cast("tconstruct:molten_copper",Some("tconstruct:seared_bricks"));
    assert!(c.tank.insert("tconstruct:molten_copper",needed-1)); let before = c.to_nbt();
    assert!(!c.tick(true)); assert_eq!(c.to_nbt(),before);
    assert!(c.tank.insert("tconstruct:molten_copper",1)); let before = c.to_nbt();
    assert!(!c.tick(false)); assert_eq!(c.to_nbt(),before);
    c = Casting::from_nbt(&c.to_nbt()).unwrap(); assert_eq!(finish(c).item.as_ref(),"tconstruct:smeltery_controller");
}
