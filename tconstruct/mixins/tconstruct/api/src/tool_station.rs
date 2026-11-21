//! Tinker station recipe dispatch over the ported tool building, repair and part-swap rules.
//! SlimeKnights (MIT; see LICENSE) defines the rules; this only picks which one applies.
//! Inputs are already resolved to materials/parts by the caller (real stacks, material recipes
//! and repair kits are not read here). Data-driven extra ingredients, modifier recipes and
//! the recipe cache are not modeled, so recipes needing them never match.
use crate::{
    materials::Definitions as Materials, modifiers::Registry, tool_build, tool_definitions::Data,
    tool_materials::MaterialVariant, tool_repair, tool_slot, tool_stack::{damage, is_broken, materials as tool_materials, Environment},
    tool_swap,
};
use pluto_lib::{Identifier, ItemSlot};

/// One non-empty station input slot, resolved by the caller.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Input {
    /// A tool part item with its material (UNKNOWN when the stack has none).
    Part { item: Identifier, material: MaterialVariant },
    /// A repair kit: the material and the kit's repair amount in ingots.
    Kit { material: Identifier, kit_amount: f32, count: i32 },
    /// An item a material recipe maps to a material (`value` of `needed` per ingot).
    Material { item: Identifier, material: Identifier, value: i32, needed: i32, count: i32 },
    /// Anything else.
    Other { item: Identifier },
}
impl Input {
    fn item(&self) -> Option<&Identifier> {
        match self { Self::Part { item, .. } | Self::Material { item, .. } | Self::Other { item } => Some(item), Self::Kit { .. } => None }
    }
}

/// What a successful lookup produces and takes.
#[derive(Debug)]
pub(crate) struct Crafted {
    pub result: ItemSlot,
    /// Items to take from each input slot, aligned with the input list.
    pub consumed: Vec<i32>,
    /// Tools taken from the tool slot (`shrinkToolSlotBy`).
    pub tool_shrink: i32,
    /// Items handed back to the player (an incremental recipe's leftover).
    pub leftover: Vec<(Identifier, u32)>,
}

#[derive(Debug)]
pub(crate) enum Outcome { NoMatch, Failure(String), Crafted(Crafted) }

/// Read-only context shared by every recipe.
pub(crate) struct Context<'a> {
    pub materials: &'a Materials,
    pub modifiers: &'a Registry,
    pub item_tags_loaded: bool,
    pub item_tags: Option<&'a pluto_lib::RegistryLookup>,
    /// Tool item id -> its definition, in recipe order. Only tools with a parts module can be built.
    pub tools: &'a [(Identifier, &'a Data)],
    /// Craft count (`result_count` capped by stack size) per tool item, from the tool item's data.
    pub item_max: &'a dyn Fn(&Identifier) -> i32,
}
impl<'a> Context<'a> {
    pub(crate) fn environment<'b>(&'b self, item: &'b Identifier, definition: &'b Data) -> Environment<'b> {
        Environment { definition: Some(definition), has_materials: definition.material_stat_types().is_some(),
            materials: self.materials, modifiers: self.modifiers, item_tags_loaded: self.item_tags_loaded,
            item: Some(item), item_tags: self.item_tags }
    }
}

/// Dispatches one station state. `tool` is the tinkerable slot, `inputs[i]` the i-th input slot
/// (None when empty). Building needs an empty tool slot; repair and swapping need a tool.
pub(crate) fn lookup(ctx: &Context, tool: Option<&ItemSlot>, inputs: &[Option<Input>]) -> Result<Outcome, String> {
    lookup_with(ctx, tool, inputs, None)
}

/// The tinker-station modifier recipes and what they need to match raw stacks.
pub(crate) struct ModifierRecipes<'a> {
    pub recipes: &'a crate::modifier_recipes::Recipes,
    pub overslime: Option<&'a crate::overslime::Recipes>,
    /// Each input slot's item and count (None when empty), independent of how it resolved.
    pub raw: &'a [Option<(Identifier, u32)>],
    /// The slot and modifier of a modifier crystal when it is the only thing on the table.
    pub crystal: Option<(usize, Identifier)>,
    pub in_tag: &'a dyn Fn(&Identifier, &Identifier) -> bool,
}

/// `lookup` plus the modifier recipes: with a tool, repair is tried first, then adding a modifier,
/// then part swapping; each answers NoMatch when it does not apply.
pub(crate) fn lookup_with(ctx: &Context, tool: Option<&ItemSlot>, inputs: &[Option<Input>], modifiers: Option<&ModifierRecipes>) -> Result<Outcome, String> {
    match tool {
        None => build(ctx, inputs),
        Some(tool) => {
            let repaired = repair(ctx, tool, inputs)?;
            if !matches!(repaired, Outcome::NoMatch) { return Ok(repaired); }
            if let Some(modifiers) = modifiers {
                let added = add_modifier(ctx, tool, modifiers)?;
                if !matches!(added, Outcome::NoMatch) { return Ok(added); }
            }
            swap(ctx, tool, inputs)
        }
    }
}

fn add_modifier(ctx: &Context, tool: &ItemSlot, modifiers: &ModifierRecipes) -> Result<Outcome, String> {
    let Some(definition) = definition_for(ctx, tool) else { return Ok(Outcome::NoMatch); };
    let env = ctx.environment(&tool.item, definition);
    // A modifier crystal alone on the table applies its modifier through the ordinary single-step recipe (ModifierRecipe.matchesCrystal).
    if let Some((slot, modifier)) = &modifiers.crystal {
        if let Some(recipe) = modifiers.recipes.find_crystal(&tool.item, modifier, modifiers.in_tag) {
            return Ok(match recipe.apply_crystal(tool, &env, modifiers.in_tag)? {
                Err(message) => Outcome::Failure(message),
                Ok(result) => {
                    let mut consumed = vec![0; modifiers.raw.len()];
                    consumed[*slot] = 1;
                    Outcome::Crafted(Crafted { result, consumed, tool_shrink: i32::try_from(recipe.max_tool_size).unwrap_or(i32::MAX), leftover: Vec::new() })
                }
            });
        }
    }
    let Some(recipe) = modifiers.recipes.find(&tool.item, modifiers.raw, modifiers.in_tag) else {
        // No modifier recipe: slime balls may fill overslime.
        let Some(recipe) = modifiers.overslime.and_then(|recipes| recipes.find(&tool.item, modifiers.raw, modifiers.in_tag)) else { return Ok(Outcome::NoMatch); };
        return Ok(match recipe.apply(tool, &env, modifiers.raw, modifiers.in_tag)? {
            Err(message) => Outcome::Failure(message),
            Ok(result) => {
                let consumed = recipe.consumption(tool, &result, &env, modifiers.raw, modifiers.in_tag).into_iter().map(|amount| i32::try_from(amount).unwrap_or(i32::MAX)).collect();
                Outcome::Crafted(Crafted { result, consumed, tool_shrink: 16, leftover: Vec::new() })
            }
        });
    };
    Ok(match recipe.apply(tool, &env, modifiers.raw, modifiers.in_tag)? {
        Err(message) => Outcome::Failure(message),
        Ok(result) => {
            let (used, leftover) = recipe.consumption(tool, &result, modifiers.raw, modifiers.in_tag);
            let consumed = used.into_iter().map(|amount| i32::try_from(amount).unwrap_or(i32::MAX)).collect();
            Outcome::Crafted(Crafted { result, consumed, tool_shrink: i32::try_from(recipe.max_tool_size).unwrap_or(i32::MAX), leftover })
        }
    })
}

fn build(ctx: &Context, inputs: &[Option<Input>]) -> Result<Outcome, String> {
    let items: Vec<Option<Identifier>> = inputs.iter().map(|input| input.as_ref().and_then(|input| input.item().cloned())).collect();
    for (tool_item, definition) in ctx.tools {
        let parts = definition.tool_parts();
        // A building recipe's parts fill every material slot of the tool (the original's ToolBuildingRecipe takes
        // its parts from the definition). A tool whose parts cover only some slots takes the rest from a cast fluid
        // in the original (the slimesuit pieces, the plate shield) and has no building recipe, so the station must
        // not invent one for it: building it here would produce a tool with the fluid's slot left unfilled.
        if parts.is_empty() || definition.material_stat_types().is_some_and(|types| types.len() != parts.len()) || !tool_build::matches(true, &items, parts, &[]) { continue; }
        let env = ctx.environment(tool_item, definition);
        let materials: Vec<MaterialVariant> = inputs.iter().take(parts.len()).map(|input| match input {
            Some(Input::Part { material, .. }) => material.clone(), _ => MaterialVariant::unknown() }).collect();
        // The recipe's result_count is 1 unless a data recipe says otherwise; no data recipe is loaded.
        return Ok(match tool_build::build(&env, &materials, &[], parts.len(), 1, (ctx.item_max)(tool_item))? {
            tool_build::Built::Failure(key) => Outcome::Failure(key),
            tool_build::Built::Success { tool, count } => {
                let mut slot = ItemSlot::new(tool_item.clone(), u32::try_from(count).unwrap_or(1));
                slot.custom_data = tool;
                tool_slot::sync_visuals(&mut slot, &env)?;
                let consumed = inputs.iter().enumerate().map(|(index, input)| i32::from(index < parts.len() && input.is_some())).collect();
                Outcome::Crafted(Crafted { result: slot, consumed, tool_shrink: 0, leftover: Vec::new() })
            }
        });
    }
    Ok(Outcome::NoMatch)
}

/// The tag query the station's recipes use: item tags from the captured snapshot, and the synthetic hook tags from the
/// tool definitions.
pub(crate) fn item_in_tag(tags: Option<&pluto_lib::RegistryLookup>, tools: &[(Identifier, &Data)], item: &Identifier, tag: &Identifier) -> bool {
    if let Some(hook) = crate::material_recipes::hook_of_tag(tag) {
        return tools.iter().find(|(id, _)| id == item).is_some_and(|(_, definition)| definition.has_hook(&hook));
    }
    tags.and_then(|tags| tags.get_tag(tag)).is_some_and(|members| members.contains(item))
}

pub(crate) fn definition_for<'a>(ctx: &'a Context, tool: &ItemSlot) -> Option<&'a Data> {
    ctx.tools.iter().find(|(item, _)| *item == tool.item).map(|(_, definition)| *definition)
}

fn repair(ctx: &Context, tool: &ItemSlot, inputs: &[Option<Input>]) -> Result<Outcome, String> {
    let Some(definition) = definition_for(ctx, tool) else { return Ok(Outcome::NoMatch); };
    let present: Vec<(usize, &Input)> = inputs.iter().enumerate().filter_map(|(index, input)| input.as_ref().map(|input| (index, input))).collect();
    let mut slots = Vec::new();
    for (_, input) in &present {
        slots.push(match input {
            Input::Kit { material, kit_amount, count } => tool_repair::Slot { material: Some(material.clone()), source: tool_repair::Source::Kit { kit_amount: *kit_amount }, count: *count },
            Input::Material { material, value, needed, count, .. } => tool_repair::Slot { material: Some(material.clone()), source: tool_repair::Source::Recipe { value: *value, needed: *needed }, count: *count },
            // An unresolvable input makes the whole recipe fail to match, as an UNKNOWN material does.
            _ => tool_repair::Slot { material: None, source: tool_repair::Source::Kit { kit_amount: 0.0 }, count: 1 },
        });
    }
    let list = tool_materials(&tool.custom_data, definition.material_stat_types().is_some());
    let env = ctx.environment(&tool.item, definition);
    // MaterialRepairToolHook.canRepairWith: the definition's own repair materials, or any modifier's.
    let can_repair = |material: &Identifier| definition.is_repair_material(&list, material) || crate::tool_stack::modifier_is_repair_material(&tool.custom_data, &env, material);
    if !tool_repair::matches(&tool.custom_data, true, &slots, can_repair) { return Ok(Outcome::NoMatch); }
    if !is_broken(&tool.custom_data) && damage(&tool.custom_data) == 0 {
        return Ok(Outcome::Failure("recipe.tconstruct.tool_repair.fully_repaired".to_owned()));
    }
    let mut result = tool.clone();
    result.count = 1;
    // MaterialRepairToolHook.getRepairAmount: the larger of the definition's amount and the modifiers'.
    let amount_for = |material: &Identifier| definition.repair_amount(&list, material, ctx.materials).max(crate::tool_stack::modifier_repair_amount(&tool.custom_data, &env, material) as f32);
    let factor = |value: f32| crate::tool_stack::repair_factor(&tool.custom_data, &env, value);
    let Some(plan) = tool_repair::plan(damage(&tool.custom_data), &slots, amount_for, factor) else { return Ok(Outcome::NoMatch); };
    tool_slot::repair_slot(&mut result, &env, plan.repaired)?;
    let mut consumed = vec![0; inputs.len()];
    for ((index, _), taken) in present.iter().zip(&plan.consumed) { consumed[*index] = *taken; }
    Ok(Outcome::Crafted(Crafted { result, consumed, tool_shrink: 1, leftover: Vec::new() }))
}

fn swap(ctx: &Context, tool: &ItemSlot, inputs: &[Option<Input>]) -> Result<Outcome, String> {
    let Some(definition) = definition_for(ctx, tool) else { return Ok(Outcome::NoMatch); };
    let parts = definition.tool_parts();
    if parts.is_empty() { return Ok(Outcome::NoMatch); }
    let Some((slot, item, material)) = inputs.iter().enumerate().find_map(|(index, input)| match input {
        Some(Input::Part { item, material }) => Some((index, item, material)), _ => None }) else { return Ok(Outcome::NoMatch); };
    // The first part must belong to the tool, and nothing else may be in the way (no extra requirements here).
    if !parts.contains(item) || inputs.iter().enumerate().any(|(index, input)| index != slot && input.is_some()) { return Ok(Outcome::NoMatch); }
    // Too many parts for the table: the original refuses with a message rather than passing.
    if parts.len() > inputs.len() { return Ok(Outcome::Failure("recipe.tconstruct.part_swapping.too_many_parts".to_owned())); }
    if material == &MaterialVariant::unknown() { return Ok(Outcome::NoMatch); }
    // The slot only matters when a tool uses a part twice: otherwise the first copy of that part.
    let index = if slot < parts.len() && &parts[slot] == item { slot } else { parts.iter().position(|part| part == item).unwrap_or(0) };
    let env = ctx.environment(&tool.item, definition);
    // MaterialCastingLookup.getItemCost needs casting recipes, which are not loaded: no part cost,
    // so a swap repairs only when the caller forces it (never here).
    let item_max = (ctx.item_max)(&tool.item);
    let max_stack = item_max.min(16);
    Ok(match tool_swap::swap_material(&tool.custom_data, i32::try_from(tool.count).unwrap_or(1), &env, material, index, 0, false, max_stack, item_max)? {
        tool_swap::Outcome::Pass => Outcome::NoMatch,
        tool_swap::Outcome::Failure(key) => Outcome::Failure(key),
        tool_swap::Outcome::Success { tool: data, size, shrink } => {
            let mut result = tool.clone();
            result.custom_data = data;
            result.count = u32::try_from(size).unwrap_or(1);
            tool_slot::sync_visuals(&mut result, &env)?;
            let mut consumed = vec![0; inputs.len()];
            consumed[slot] = 1;
            Outcome::Crafted(Crafted { result, consumed, tool_shrink: shrink, leftover: Vec::new() })
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::{Material, Rarity};
    use serde_json::json;
    struct Always;
    impl crate::materials::Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &serde_json::Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn variant(text: &str) -> MaterialVariant { MaterialVariant::parse(text).unwrap() }
    struct World { materials: Materials, registry: Registry, pickaxe: Data }
    fn world() -> World {
        let pickaxe = Data::decode(&json!({"modules":[
            {"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle"]},
            {"type":"tconstruct:is_effective","predicate_type":"mantle:tag","tag":"minecraft:mineable/pickaxe"}
        ]})).unwrap();
        let mut materials = Materials::default();
        materials.stats = crate::material_stats::load(&["test:wood", "test:stone"].map(|name| pluto_lib::JsonResourceStack { id: id(name), layers: vec![
            Ok(json!({"stats":{"tconstruct:head":{"durability":40,"mining_speed":3},"tconstruct:handle":{"durability":0.5}}}))] }), &mut materials.problems);
        for name in ["test:wood", "test:stone"] {
            materials.materials.insert(id(name), Material { id: id(name), tier: 1, sort_order: 1, rarity: Rarity::Common, craftable: true, hidden: false });
        }
        materials.server_loaded = true;
        let mut registry = Registry::default();
        registry.reload(&[], &Always);
        World { materials, registry, pickaxe }
    }
    fn part(item: &str, material: &str) -> Option<Input> { Some(Input::Part { item: id(item), material: variant(material) }) }
    #[test]
    fn parts_build_a_tool_which_then_repairs_and_swaps_through_the_same_dispatch() {
        let world = world();
        let tools = [(id("tconstruct:pickaxe"), &world.pickaxe)];
        let item_max = |_: &Identifier| 1;
        let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
        // Build: head + handle in slot order, binding slot empty.
        let inputs = [part("tconstruct:pick_head", "test:wood"), part("tconstruct:tool_handle", "test:wood"), None];
        let Outcome::Crafted(built) = lookup(&ctx, None, &inputs).unwrap() else { panic!("expected a built tool") };
        assert_eq!((built.result.item.to_string(), built.result.count, built.consumed.clone(), built.tool_shrink),
            ("tconstruct:pickaxe".to_owned(), 1, vec![1, 1, 0], 0));
        assert!(built.result.max_damage.is_some() && built.result.tool.is_some());
        // Wrong part order does not match anything.
        let wrong = [part("tconstruct:tool_handle", "test:wood"), part("tconstruct:pick_head", "test:wood"), None];
        assert!(matches!(lookup(&ctx, None, &wrong).unwrap(), Outcome::NoMatch));
        // Undamaged tool + new head: swap to stone (material changed, no repair).
        let swap_inputs = [part("tconstruct:pick_head", "test:stone"), None, None];
        let Outcome::Crafted(swapped) = lookup(&ctx, Some(&built.result), &swap_inputs).unwrap() else { panic!("expected a swap") };
        assert_eq!(swapped.consumed, vec![1, 0, 0]);
        assert_eq!(crate::tool_stack::materials(&swapped.result.custom_data, true).get(0).to_string(), "test:stone");
        // A damaged tool with an undamaged-looking swap candidate still swaps; with a repair kit it repairs.
        let mut damaged = swapped.result.clone();
        tool_slot::damage_slot(&mut damaged, &ctx.environment(&id("tconstruct:pickaxe"), &world.pickaxe), 5, true, true, None, &mut || 1.0).unwrap();
        let kit = [Some(Input::Kit { material: id("test:stone"), kit_amount: 3.0, count: 8 }), None, None];
        let Outcome::Crafted(repaired) = lookup(&ctx, Some(&damaged), &kit).unwrap() else { panic!("expected a repair") };
        assert_eq!(repaired.tool_shrink, 1);
        assert!(repaired.consumed[0] >= 1);
        assert_eq!(repaired.result.damage, 0, "the kit repaired the head's worth");
    }
    #[test]
    fn full_health_repairs_fail_and_mixed_or_unknown_inputs_do_not_match() {
        let world = world();
        let tools = [(id("tconstruct:pickaxe"), &world.pickaxe)];
        let item_max = |_: &Identifier| 1;
        let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
        let Outcome::Crafted(built) = lookup(&ctx, None, &[part("tconstruct:pick_head", "test:wood"), part("tconstruct:tool_handle", "test:wood")]).unwrap() else { panic!() };
        let kit = |material: &str| Some(Input::Kit { material: id(material), kit_amount: 3.0, count: 1 });
        // Undamaged: repair never matches (it defers), nothing else applies.
        assert!(matches!(lookup(&ctx, Some(&built.result), &[kit("test:wood")]).unwrap(), Outcome::NoMatch));
        let mut hurt = built.result.clone();
        tool_slot::damage_slot(&mut hurt, &ctx.environment(&id("tconstruct:pickaxe"), &world.pickaxe), 3, true, true, None, &mut || 1.0).unwrap();
        // Two different materials, an unrepairable material and a stray item all fail to match.
        assert!(matches!(lookup(&ctx, Some(&hurt), &[kit("test:wood"), kit("test:stone")]).unwrap(), Outcome::NoMatch));
        assert!(matches!(lookup(&ctx, Some(&hurt), &[kit("test:other")]).unwrap(), Outcome::NoMatch));
        assert!(matches!(lookup(&ctx, Some(&hurt), &[Some(Input::Other { item: id("minecraft:dirt") })]).unwrap(), Outcome::NoMatch));
        // A tool the dispatcher has no definition for is left to other recipes.
        let alien = ItemSlot::new(id("minecraft:stick"), 1);
        assert!(matches!(lookup(&ctx, Some(&alien), &[kit("test:wood")]).unwrap(), Outcome::NoMatch));
    }
}
