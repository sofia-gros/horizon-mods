//! Domain checks for trader.rs: WorldEvents.wanderingTrades' ancient-tool listing.
use pluto_lib::{Identifier, TradeRandom, TradeWorld};
use std::collections::VecDeque;

/// A trade RNG that plays back scripted draws, then 0; a draw outside the asked bound fails,
/// so a test pins the number and order of the listing's rolls.
struct Scripted { draws: VecDeque<i32> }
impl Scripted {
    fn new(draws: &[i32]) -> Self { Self { draws: draws.iter().copied().collect() } }
    fn zeros() -> Self { Self::new(&[]) }
}
impl TradeRandom for Scripted {
    fn next_int(&mut self, bound: i32) -> i32 {
        let draw = self.draws.pop_front().unwrap_or(0);
        assert!((0..bound).contains(&draw), "scripted draw {draw} outside 0..{bound}");
        draw
    }
    fn next_float(&mut self) -> f32 { 0.0 }
    fn next_double(&mut self) -> f64 { 0.0 }
}

struct FakeWorld { tag: Option<Vec<String>> }
impl TradeWorld for FakeWorld {
    fn item_tag(&self, tag: &str) -> Option<Vec<String>> {
        assert_eq!(tag, "tconstruct:modifiable/wandering_trader");
        self.tag.clone()
    }
    fn item_exists(&self, _: &str) -> bool { true }
}

fn definitions(tools: Vec<(Identifier, crate::tool_definitions::Data)>) -> crate::tool_definitions::Definitions {
    crate::tool_definitions::Definitions { data: tools.into_iter().collect(), ..Default::default() }
}

#[test]
fn the_configured_weight_counts_the_listings_pool_copies() {
    assert!(crate::trader::factories(0).is_empty());
    assert_eq!(crate::trader::factories(1).len(), 1);
    assert_eq!(crate::trader::factories(6).len(), 6);
}

#[test]
fn a_zero_roll_listing_sells_the_first_tag_tool_built_from_the_first_ancient_materials() {
    let crate::original_data::World { materials, registry, tools } = crate::original_data::world();
    let definitions = definitions(tools);
    // The shipped tconstruct:modifiable/wandering_trader tag, in file order.
    let tag = ["tconstruct:melting_pan", "tconstruct:war_pick", "tconstruct:battlesign", "tconstruct:swasher"]
        .into_iter().map(str::to_owned).collect();
    let trade_world = FakeWorld { tag: Some(tag) };
    let mut random = Scripted::zeros();
    let offer = crate::trader::listing(&trade_world, &materials, &registry, &definitions, None, &mut random)
        .expect("the listing builds an offer from the shipped data");
    // Draw 0 picks the tag's first tool; the offer is the listing's fixed shape.
    assert_eq!(offer.sell.item, "tconstruct:melting_pan");
    assert_eq!(offer.sell.count, 1);
    assert_eq!(offer.max_uses, 1);
    assert_eq!(offer.experience, 15);
    assert_eq!(offer.price_multiplier, 1.0);
    assert!(offer.second_buy.is_none());
    assert_eq!(offer.first_buy.item, "minecraft:emerald");
    // Every part took the first ancient candidate for its stat type (draw 0 in registry
    // order), and the price roll added 0 to round(average tier x 6).
    let definition = &definitions.data[&Identifier::parse("tconstruct:melting_pan").unwrap()];
    let stat_types = definition.material_stat_types().unwrap();
    let built = crate::tool_stack::materials(&offer.sell.custom_data, true);
    let built: Vec<Identifier> = built.as_slice().iter().map(|variant| variant.material_id().unwrap()).collect();
    assert_eq!(built.len(), stat_types.len());
    let excluded = materials.tags.get(&Identifier::parse("tconstruct:exclude_from_loot").unwrap());
    let mut tier_sum = 0;
    for (index, (stat, _)) in stat_types.iter().enumerate() {
        let first = materials.materials.keys()
            .find(|id| !excluded.is_some_and(|list| list.contains(id)) && materials.get_stats(id, stat).is_some())
            .expect("a shipped ancient candidate");
        assert_eq!(&built[index], first, "part {index}");
        tier_sum += materials.materials[first].tier;
    }
    let tier = tier_sum as f32 / stat_types.len() as f32;
    assert_eq!(offer.first_buy.count, (tier * 6.0).round() as u32);
}

#[test]
fn the_listing_declines_without_a_tag_or_for_a_pick_that_is_not_a_tool() {
    let crate::original_data::World { materials, registry, tools } = crate::original_data::world();
    let definitions = definitions(tools);
    let mut random = Scripted::zeros();
    let no_tag = FakeWorld { tag: None };
    assert!(crate::trader::listing(&no_tag, &materials, &registry, &definitions, None, &mut random).is_none());
    let empty = FakeWorld { tag: Some(Vec::new()) };
    assert!(crate::trader::listing(&empty, &materials, &registry, &definitions, None, &mut random).is_none());
    let stick = FakeWorld { tag: Some(vec!["minecraft:stick".to_owned()]) };
    assert!(crate::trader::listing(&stick, &materials, &registry, &definitions, None, &mut random).is_none());
}

#[test]
fn the_emerald_price_is_the_tier_formula() {
    // Tier 2 (the listing's no-materials fallback): round(2 x 6) + a roll of 0..=round(2 x 2).
    assert_eq!(crate::trader::emerald_cost(2.0, &mut Scripted::new(&[0])), 12);
    assert_eq!(crate::trader::emerald_cost(2.0, &mut Scripted::new(&[4])), 16);
    // Tier 3.5: round(3.5 x 6) + a roll of 0..=round(2 x 3.5).
    assert_eq!(crate::trader::emerald_cost(3.5, &mut Scripted::new(&[7])), 28);
}

#[test]
fn the_ancient_pick_skips_loot_excluded_materials() {
    let mut world = crate::original_data::world();
    world.materials.load_tags(&crate::original_data::stacks("tags/materials"));
    let stat = Identifier::parse("tconstruct:head").unwrap();
    let excluded = world.materials.tags.get(&Identifier::parse("tconstruct:exclude_from_loot").unwrap());
    assert!(excluded.is_some_and(|list| !list.is_empty()), "the shipped exclusion tag is populated");
    let candidates: Vec<&Identifier> = world.materials.materials.keys()
        .filter(|id| !excluded.is_some_and(|list| list.contains(id)) && world.materials.get_stats(id, &stat).is_some())
        .collect();
    assert!(!candidates.is_empty());
    // The last candidate in registry order is still an eligible material, never an excluded one.
    let last = candidates.len() - 1;
    let pick = crate::trader::ancient_material(&world.materials, &stat, &mut Scripted::new(&[last as i32])).unwrap();
    assert_eq!(&pick, candidates[last]);
    assert!(!excluded.is_some_and(|list| list.contains(&pick)));
}
