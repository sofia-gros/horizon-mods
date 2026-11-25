//! WorldEvents.wanderingTrades and AncientToolItemListing (SlimeKnights, MIT): the wandering
//! trader's rare pool gains `wandererAncientToolWeight` copies of a listing that sells a tool
//! built from random ancient materials for emeralds priced by its average material tier.
use pluto_lib::{Identifier, RegistryLookup, StackData, TradeOffer, TradeOfferContext, TradeOfferFactory, TradeOfferHelper, TradeRandom, TradeWorld};

/// TinkerTags.Items.TRADER_TOOLS: the tools the listing can sell.
const TRADER_TOOLS_TAG: &str = "tconstruct:modifiable/wandering_trader";

pub(crate) fn register() {
    // The provider runs for each pool snapshot, so it reads the configured weight per snapshot
    // as the original reads it per WandererTradesEvent; a reload needs no re-registration.
    if let Err(error) = TradeOfferHelper::register_wandering_trader_offers(2, || {
        factories(crate::config::wanderer_ancient_tool_weight())
    }) {
        eprintln!("Tinkers: wandering trader offers not registered: {error}");
    }
}

/// The listing's pool entries for a configured weight: that many copies of the one factory,
/// as the original adds the listing instance `weight` times; a weight of 0 adds none.
pub(crate) fn factories(weight: i32) -> Vec<TradeOfferFactory> {
    (0..weight.max(0)).map(|_| TradeOfferFactory::new(ancient_tool_offer)).collect()
}

/// AncientToolItemListing.getOffer against the live registries; a registry that is not
/// loaded declines the listing rather than fabricating a tool.
fn ancient_tool_offer(context: &TradeOfferContext<'_>, random: &mut dyn TradeRandom) -> Result<Option<TradeOffer>, String> {
    let materials = crate::common::register().read().map_err(|_| "materials registry poisoned")?;
    if !materials.server_loaded { return Ok(None); }
    let Some(modifiers) = crate::common::server_modifiers() else { return Ok(None) };
    let modifiers = modifiers.read().map_err(|_| "modifier registry poisoned")?;
    if !modifiers.definitions_loaded { return Ok(None); }
    let definitions = crate::tool_definitions::register().read().map_err(|_| "tool definitions poisoned")?;
    let tags = crate::common::item_snapshot();
    Ok(listing(context.world, &materials, &modifiers, &definitions, tags.as_deref(), random))
}

/// The listing itself: pick a random tool from the trader tag, build it from random ancient
/// materials, and price it in emeralds. None when the tag pick is not a modifiable tool here
/// or the tool cannot be built, as the original returns no offer then.
pub(crate) fn listing(world: &dyn TradeWorld, materials: &crate::materials::Definitions, modifiers: &crate::modifiers::Registry,
    definitions: &crate::tool_definitions::Definitions, item_tags: Option<&RegistryLookup>, random: &mut dyn TradeRandom) -> Option<TradeOffer> {
    let tag = world.item_tag(TRADER_TOOLS_TAG)?;
    if tag.is_empty() { return None; }
    let picked = &tag[random.next_int(tag.len() as i32) as usize];
    let item = Identifier::parse(picked).ok()?;
    let definition = definitions.data.get(&item)?;
    let (sell, tier) = build_ancient_tool(materials, modifiers, definition, item_tags, &item, random)?;
    Some(TradeOffer {
        first_buy: StackData { item: "minecraft:emerald".to_owned(), count: emerald_cost(tier, random), ..Default::default() },
        second_buy: None,
        sell,
        max_uses: 1,
        experience: 15,
        price_multiplier: 1.0,
    })
}

/// AncientToolItemListing's emerald price: round(tier x 6) plus a roll of round(2 x tier) + 1.
pub(crate) fn emerald_cost(tier: f32, random: &mut dyn TradeRandom) -> u32 {
    let cost = (tier * 6.0).round() as i32 + random.next_int((tier * 2.0).round() as i32 + 1);
    cost.max(0) as u32
}

/// ToolBuildHandler.buildToolRandomMaterials with RandomMaterial.ancient(): one random
/// ancient material per stat type of the definition, then the tool built from them.
/// Returns the built stack and the average material tier (2 when the tool has no materials,
/// the listing's own fallback).
fn build_ancient_tool(materials: &crate::materials::Definitions, modifiers: &crate::modifiers::Registry,
    definition: &crate::tool_definitions::Data, item_tags: Option<&RegistryLookup>, item: &Identifier,
    random: &mut dyn TradeRandom) -> Option<(StackData, f32)> {
    let stat_types = definition.material_stat_types()?;
    let env = crate::tool_stack::Environment {
        definition: Some(definition), has_materials: true, materials, modifiers,
        item_tags_loaded: item_tags.is_some(), item: Some(item), item_tags,
    };
    let mut variants = Vec::new();
    let mut tier_sum = 0;
    for (stat, _scale) in stat_types {
        let chosen = ancient_material(materials, stat, random)?;
        tier_sum += materials.materials.get(&chosen)?.tier;
        variants.push(crate::tool_materials::MaterialVariant::parse(&chosen.to_string())?);
    }
    let tier = if variants.is_empty() { 2.0 } else { tier_sum as f32 / variants.len() as f32 };
    let tool = crate::tool_build::create_tool(&env, &crate::tool_materials::MaterialList::from_variants(variants), false).ok()?;
    Some((StackData { item: item.to_string(), count: 1, custom_data: tool, ..Default::default() }, tier))
}

/// RandomMaterial.ancient: any tier, hidden allowed, never a material in
/// `tconstruct:exclude_from_loot`, with stats for the requested type; a uniform pick, falling
/// back to the first material with the stat. The same pick slime_entities makes for slime
/// gear, here over the trade RNG.
pub(crate) fn ancient_material(materials: &crate::materials::Definitions, stat: &Identifier, random: &mut dyn TradeRandom) -> Option<Identifier> {
    let excluded = materials.tags.get(&Identifier::parse("tconstruct:exclude_from_loot").expect("tag id"));
    let choices: Vec<&Identifier> = materials.materials.keys()
        .filter(|id| !excluded.is_some_and(|list| list.contains(id)) && materials.get_stats(id, stat).is_some())
        .collect();
    if choices.is_empty() { return materials.first_with_stat(stat).map(|material| material.id.clone()); }
    Some(choices[random.next_int(choices.len() as i32) as usize].clone())
}
