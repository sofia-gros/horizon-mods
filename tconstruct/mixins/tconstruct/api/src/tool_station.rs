//! Tinker station recipe dispatch over the ported tool building, repair and part-swap rules.
//! SlimeKnights (MIT; see LICENSE) defines the rules; this only picks which one applies.
//! Inputs are already resolved to materials/parts by the caller (real stacks, material recipes
//! and repair kits are not read here). Data-driven extra ingredients, modifier recipes and
//! the recipe cache are not modeled, so recipes needing them never match.
use crate::{
    materials::Definitions as Materials, modifiers::Registry, tool_build, tool_damage, tool_definitions::Data,
    tool_materials::MaterialVariant, tool_repair, tool_slot, tool_stack::{damage, durability, is_broken, materials as tool_materials, Environment},
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
    /// An item a material recipe maps to a material; the recipe itself rides along because
    /// the swapping recipes price and refund from it, not just from its value and needed.
    Material { item: Identifier, material: Identifier, recipe: crate::material_recipes::Recipe, count: i32 },
    /// A tinkerable tool stack in an input slot (the sacrifice of tool material swapping).
    Tool { stack: Box<ItemSlot> },
    /// Anything else.
    Other { item: Identifier },
}
impl Input {
    fn item(&self) -> Option<&Identifier> {
        match self {
            Self::Part { item, .. } | Self::Material { item, .. } | Self::Other { item } => Some(item),
            Self::Tool { stack } => Some(&stack.item),
            Self::Kit { .. } => None,
        }
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
    lookup_with(ctx, tool, inputs, None, None)
}

/// What the material swapping recipes (and the part swap's repair) read beyond the resolved
/// inputs: the casting cost of a part item and the loaded swapping recipes.
pub(crate) struct SwapData<'a> {
    /// MaterialCastingLookup.getItemCost: a part item's casting cost (0 when unregistered).
    pub part_cost: &'a dyn Fn(&Identifier) -> i32,
    pub recipes: Option<&'a crate::swapping_recipes::Recipes>,
    /// Each input slot's item and count (None when empty), for extra requirements.
    pub raw: &'a [Option<(Identifier, u32)>],
    pub in_tag: &'a dyn Fn(&Identifier, &Identifier) -> bool,
}

/// The tinker-station modifier recipes and what they need to match raw stacks.
pub(crate) struct ModifierRecipes<'a> {
    pub recipes: &'a crate::modifier_recipes::Recipes,
    pub overslime: Option<&'a crate::overslime::Recipes>,
    pub modifier_repairs: Option<&'a crate::modifier_repair::Recipes>,
    pub damaging: Option<&'a crate::damaging::Recipes>,
    pub dyeing: Option<&'a crate::dyeing::Recipes>,
    /// Each input slot's item and count (None when empty), independent of how it resolved.
    pub raw: &'a [Option<(Identifier, u32)>],
    /// The slot and modifier of a modifier crystal when it is the only thing on the table.
    pub crystal: Option<(usize, Identifier)>,
    pub in_tag: &'a dyn Fn(&Identifier, &Identifier) -> bool,
}

/// `lookup` plus the modifier recipes: with a tool, repair is tried first, then adding a modifier,
/// then part swapping, then the material swapping recipes; each answers NoMatch when it does not apply.
pub(crate) fn lookup_with(ctx: &Context, tool: Option<&ItemSlot>, inputs: &[Option<Input>], modifiers: Option<&ModifierRecipes>, swap_data: Option<&SwapData>) -> Result<Outcome, String> {
    match tool {
        None => build(ctx, inputs),
        Some(tool) => {
            let repaired = repair(ctx, tool, inputs)?;
            if !matches!(repaired, Outcome::NoMatch) { return Ok(repaired); }
            if let Some(modifiers) = modifiers {
                let added = add_modifier(ctx, tool, modifiers)?;
                if !matches!(added, Outcome::NoMatch) { return Ok(added); }
            }
            let part_cost: &dyn Fn(&Identifier) -> i32 = match swap_data {
                Some(data) => data.part_cost,
                None => &|_| 0,
            };
            let swapped = swap(ctx, tool, inputs, part_cost)?;
            if !matches!(swapped, Outcome::NoMatch) { return Ok(swapped); }
            match swap_data {
                Some(data) => swap_recipes(ctx, tool, inputs, data),
                None => Ok(Outcome::NoMatch),
            }
        }
    }
}

fn add_modifier(ctx: &Context, tool: &ItemSlot, modifiers: &ModifierRecipes) -> Result<Outcome, String> {
    let Some(definition) = definition_for(ctx, tool) else { return Ok(Outcome::NoMatch); };
    let env = ctx.environment(&tool.item, definition);
    // A modifier repair recipe (tasty and its kin) is tried before adding: its ingredient
    // feeds the tool's repair, it is never a modifier recipe's input.
    if let Some(recipes) = modifiers.modifier_repairs {
        if let Some(attempt) = recipes.attempt(tool, &env, modifiers.raw, modifiers.in_tag) {
            return Ok(match attempt {
                Err(message) => Outcome::Failure(message),
                Ok((result, consumed)) => Outcome::Crafted(Crafted {
                    result,
                    consumed: consumed.into_iter().map(|amount| i32::try_from(amount).unwrap_or(i32::MAX)).collect(),
                    tool_shrink: 1,
                    leftover: Vec::new(),
                }),
            });
        }
    }
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
        if let Some(recipe) = modifiers.overslime.and_then(|recipes| recipes.find(&tool.item, modifiers.raw, modifiers.in_tag)) {
            return Ok(match recipe.apply(tool, &env, modifiers.raw, modifiers.in_tag)? {
                Err(message) => Outcome::Failure(message),
                Ok(result) => {
                    let consumed = recipe.consumption(tool, &result, &env, modifiers.raw, modifiers.in_tag).into_iter().map(|amount| i32::try_from(amount).unwrap_or(i32::MAX)).collect();
                    Outcome::Crafted(Crafted { result, consumed, tool_shrink: 16, leftover: Vec::new() })
                }
            });
        }
        // Nor overslime: dyes may recolor dyeable gear.
        if let Some(recipes) = modifiers.dyeing {
            if let Some(attempt) = recipes.attempt(tool, &env, modifiers.raw, modifiers.in_tag) {
                return Ok(match attempt {
                    Err(message) => Outcome::Failure(message),
                    Ok((result, consumed)) => Outcome::Crafted(Crafted {
                        result,
                        consumed: consumed.into_iter().map(|amount| i32::try_from(amount).unwrap_or(i32::MAX)).collect(),
                        tool_shrink: 1,
                        leftover: Vec::new(),
                    }),
                });
            }
        }
        // Nor dyeing: a damaging recipe may still take the tool.
        if let Some(recipes) = modifiers.damaging {
            if let Some(attempt) = recipes.attempt(tool, &env, modifiers.raw, modifiers.in_tag) {
                return Ok(match attempt {
                    Err(message) => Outcome::Failure(message),
                    Ok((result, consumed)) => Outcome::Crafted(Crafted {
                        result,
                        consumed: consumed.into_iter().map(|amount| i32::try_from(amount).unwrap_or(i32::MAX)).collect(),
                        tool_shrink: 1,
                        leftover: Vec::new(),
                    }),
                });
            }
        }
        return Ok(Outcome::NoMatch);
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
            Input::Material { material, recipe, count, .. } => tool_repair::Slot { material: Some(material.clone()), source: tool_repair::Source::Recipe { value: recipe.value, needed: recipe.needed }, count: *count },
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

fn swap(ctx: &Context, tool: &ItemSlot, inputs: &[Option<Input>], part_cost: &dyn Fn(&Identifier) -> i32) -> Result<Outcome, String> {
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
    // TinkerStationPartSwapping passes MaterialCastingLookup.getItemCost(part): the part's
    // casting cost, which swap_material turns into repair against the new material.
    let item_max = (ctx.item_max)(&tool.item);
    let max_stack = item_max.min(16);
    Ok(match tool_swap::swap_material(&tool.custom_data, i32::try_from(tool.count).unwrap_or(1), &env, material, index, part_cost(item), false, max_stack, item_max)? {
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

const TOO_MANY_PARTS: &str = "recipe.tconstruct.part_swapping.too_many_parts";
const INVALID_MATERIAL: &str = "recipe.tconstruct.part_swapping.invalid_material";
const NO_MODIFIERS: &str = "recipe.tconstruct.part_swapping.no_modifiers";

/// MaterialRegistry.canRepair: the stat type's default stats implement IRepairableMaterialStats —
/// HeadMaterialStats, LimbMaterialStats, the five PlatingMaterialStats types, SkullStats,
/// SlimeStats, and RepairStats' ribcage, shell and laces.
fn can_repair_stat_type(stat_type: &Identifier) -> bool {
    matches!(stat_type.to_string().as_str(),
        "tconstruct:head" | "tconstruct:limb" | "tconstruct:plating_helmet" | "tconstruct:plating_chestplate"
        | "tconstruct:plating_leggings" | "tconstruct:plating_boots" | "tconstruct:plating_shield"
        | "tconstruct:skull" | "tconstruct:slime" | "tconstruct:ribcage" | "tconstruct:shell" | "tconstruct:laces")
}

/// ModifierRecipe.checkMatch over a swapping recipe's extra requirements, with the part slot
/// already used: each requirement takes a distinct remaining slot (item match, count covering
/// the amount), and no other non-empty input may be left over. Answers the takes per slot.
fn match_extra(
    extra: &[crate::swapping_recipes::SizedIngredient], inputs: &[Option<Input>], raw: &[Option<(Identifier, u32)>],
    in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, part_slot: usize,
) -> Option<Vec<(usize, i32)>> {
    let mut used = vec![false; inputs.len()];
    used[part_slot] = true;
    let mut takes = Vec::new();
    for requirement in extra {
        let slot = (0..inputs.len()).find(|&i| {
            !used[i] && inputs[i].as_ref().and_then(|input| input.item()).is_some_and(|item| requirement.ingredient.matches(item, in_tag))
                && raw.get(i).and_then(|slot| slot.as_ref()).is_some_and(|(_, count)| *count >= u32::try_from(requirement.amount).unwrap_or(u32::MAX))
        })?;
        used[slot] = true;
        takes.push((slot, requirement.amount));
    }
    if inputs.iter().enumerate().any(|(i, input)| !used[i] && input.is_some()) { return None; }
    Some(takes)
}

/// Runs the loaded material swapping recipes: a recipe whose matches() half fails answers
/// NoMatch; once matched, the getValidatedResult half answers the failure or the crafted swap.
fn swap_recipes(ctx: &Context, tool: &ItemSlot, inputs: &[Option<Input>], data: &SwapData) -> Result<Outcome, String> {
    use crate::swapping_recipes::Recipe;
    let Some(recipes) = data.recipes else { return Ok(Outcome::NoMatch); };
    let Some(definition) = definition_for(ctx, tool) else { return Ok(Outcome::NoMatch); };
    let stat_types = definition.material_stat_types().unwrap_or(&[]);
    let env = ctx.environment(&tool.item, definition);
    let item_max = (ctx.item_max)(&tool.item);
    let tool_count = i32::try_from(tool.count).unwrap_or(1);
    // The swap tail shared by all three recipe kinds.
    let finish = |swapped: tool_swap::Outcome, consumed: Vec<i32>, leftover: Vec<(Identifier, u32)>| -> Result<Outcome, String> {
        Ok(match swapped {
            tool_swap::Outcome::Pass => Outcome::NoMatch,
            tool_swap::Outcome::Failure(key) => Outcome::Failure(key),
            tool_swap::Outcome::Success { tool: data, size, shrink } => {
                let mut result = tool.clone();
                result.custom_data = data;
                result.count = u32::try_from(size).unwrap_or(1);
                tool_slot::sync_visuals(&mut result, &env)?;
                Outcome::Crafted(Crafted { result, consumed, tool_shrink: shrink, leftover })
            }
        })
    };
    let raw_count = |slot: usize| data.raw.get(slot).and_then(|entry| entry.as_ref()).map_or(0, |(_, count)| *count);
    // The predicate snapshot for material value swapping: the station's own definitions,
    // with the global recipe/casting/item snapshots where they have loaded.
    let loaded = crate::shaped_materials::Data {
        definitions: ctx.materials.clone(),
        material_recipes: crate::common::material_recipes().and_then(|recipes| recipes.read().ok().map(|recipes| recipes.clone())).unwrap_or_default(),
        casting: crate::common::smeltery_data().and_then(|data| data.read().ok().map(|data| data.recipes.material_casting.clone())).unwrap_or_default(),
        items: crate::common::item_snapshot(),
    };
    for recipe in recipes.recipes() {
        if !recipe.tools().matches(&tool.item, data.in_tag) { continue; }
        let outcome = match recipe {
            Recipe::Fixed { ingredient, material, indices, repair_value, .. } => {
                if indices[0] < 0 || indices[0] as usize >= stat_types.len() { continue; }
                let Some(slot) = inputs.iter().position(|input| {
                    input.as_ref().and_then(|input| input.item()).is_some_and(|item| ingredient.ingredient.matches(item, data.in_tag))
                }) else { continue; };
                if raw_count(slot) < u32::try_from(ingredient.amount).unwrap_or(u32::MAX) { continue; }
                let Some(extra_takes) = match_extra(recipe.extra(), inputs, data.raw, data.in_tag, slot) else { continue; };
                if stat_types.len() > inputs.len() { return Ok(Outcome::Failure(TOO_MANY_PARTS.to_owned())); }
                let index = if indices[1..].contains(&(slot as i32)) { slot } else { indices[0] as usize };
                if index < stat_types.len() {
                    let valid = material.material_id().is_some_and(|id| ctx.materials.get_stats(&id, &stat_types[index].0).is_some());
                    if !valid { return Ok(Outcome::Failure(INVALID_MATERIAL.to_owned())); }
                }
                let mut consumed = vec![0; inputs.len()];
                consumed[slot] = ingredient.amount;
                for (extra_slot, amount) in extra_takes { consumed[extra_slot] = amount; }
                let swapped = tool_swap::swap_material(&tool.custom_data, tool_count, &env, material, index, *repair_value, false, recipe.max_stack_size(), item_max)?;
                finish(swapped, consumed, Vec::new())?
            }
            Recipe::Value { predicate, cost, indices, .. } => {
                if indices[0] < 0 || indices[0] as usize >= stat_types.len() { continue; }
                // MaterialValueSwappingRecipe.isMaterial: a material recipe whose variant
                // satisfies the predicate, with enough items to pay the cost.
                let Some((slot, material_recipe)) = inputs.iter().enumerate().find_map(|(i, input)| match input {
                    Some(Input::Material { recipe, count, .. })
                        if *count >= recipe.items_used(*cost) && crate::shaped_materials::predicate_matches(predicate, &recipe.material, &loaded) => Some((i, recipe)),
                    _ => None,
                }) else { continue; };
                let material_recipe = material_recipe.clone();
                let Some(extra_takes) = match_extra(recipe.extra(), inputs, data.raw, data.in_tag, slot) else { continue; };
                if stat_types.len() > inputs.len() { return Ok(Outcome::Failure(TOO_MANY_PARTS.to_owned())); }
                let variant = material_recipe.material.clone();
                // MaterialIndexSwappingRecipe: an UNKNOWN variant passes the recipe by.
                if variant == MaterialVariant::unknown() { continue; }
                let index = if slot < stat_types.len() && indices[1..].contains(&(slot as i32)) { slot } else { indices[0] as usize };
                if index < stat_types.len() {
                    let valid = variant.material_id().is_some_and(|id| ctx.materials.get_stats(&id, &stat_types[index].0).is_some());
                    if !valid { return Ok(Outcome::Failure(INVALID_MATERIAL.to_owned())); }
                }
                let mut consumed = vec![0; inputs.len()];
                consumed[slot] = material_recipe.items_used(*cost);
                for (extra_slot, amount) in extra_takes { consumed[extra_slot] = amount; }
                let leftover = match material_recipe.leftover_for(*cost) {
                    Some((crate::material_recipes::LeftoverSource::Item(item), count)) => vec![(item.clone(), count)],
                    Some((crate::material_recipes::LeftoverSource::Tag(tag), count)) => loaded.items.as_ref()
                        .and_then(|items| items.get_tag(tag)).and_then(|members| members.iter().next().cloned())
                        .map(|item| vec![(item, count)]).unwrap_or_default(),
                    None => Vec::new(),
                };
                let swapped = tool_swap::swap_material(&tool.custom_data, tool_count, &env, &variant, index, *cost, false, recipe.max_stack_size(), item_max)?;
                finish(swapped, consumed, leftover)?
            }
            Recipe::Tool { .. } => {
                if stat_types.is_empty() { continue; }
                let Some((slot, sacrifice)) = inputs.iter().enumerate().find_map(|(i, input)| match input {
                    Some(Input::Tool { stack }) if stack.item == tool.item => Some((i, stack)),
                    _ => None,
                }) else { continue; };
                let sacrifice = (**sacrifice).clone();
                let Some(extra_takes) = match_extra(recipe.extra(), inputs, data.raw, data.in_tag, slot) else { continue; };
                if stat_types.len() > inputs.len() { return Ok(Outcome::Failure(TOO_MANY_PARTS.to_owned())); }
                if !crate::tool_stack::upgrades(&sacrifice.custom_data).entries().is_empty() {
                    return Ok(Outcome::Failure(NO_MODIFIERS.to_owned()));
                }
                let index = if slot >= stat_types.len() { 0 } else { slot };
                let may_repair = can_repair_stat_type(&stat_types[index].0);
                let material = tool_materials(&sacrifice.custom_data, env.has_materials).get(index as i32);
                let swapped = tool_swap::swap_material(&tool.custom_data, tool_count, &env, &material, index, 0, may_repair, recipe.max_stack_size(), item_max)?;
                // A repairable swap also repairs by the sacrifice's remaining durability, plus 5%.
                let swapped = match swapped {
                    tool_swap::Outcome::Success { tool: mut data, size, shrink } if may_repair && damage(&data) > 0 => {
                        let extra = durability(&sacrifice.custom_data) - damage(&sacrifice.custom_data) + durability(&data) / 20;
                        tool_damage::repair(&mut data, extra);
                        tool_swap::Outcome::Success { tool: data, size, shrink }
                    }
                    other => other,
                };
                let mut consumed = vec![0; inputs.len()];
                consumed[slot] = 1;
                for (extra_slot, amount) in extra_takes { consumed[extra_slot] = amount; }
                finish(swapped, consumed, Vec::new())?
            }
        };
        if !matches!(outcome, Outcome::NoMatch) { return Ok(outcome); }
    }
    Ok(Outcome::NoMatch)
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
    fn build(ctx: &Context, head: &str, handle: &str) -> ItemSlot {
        let inputs = [part("tconstruct:pick_head", head), part("tconstruct:tool_handle", handle), None];
        let Outcome::Crafted(built) = lookup(ctx, None, &inputs).unwrap() else { panic!("expected a built tool") };
        built.result
    }
    fn hurt(ctx: &Context, world: &World, tool: &mut ItemSlot, amount: i32) {
        tool_slot::damage_slot(tool, &ctx.environment(&id("tconstruct:pickaxe"), &world.pickaxe), amount, true, true, None, &mut || 1.0).unwrap();
    }
    #[test]
    fn part_swap_repairs_by_the_parts_casting_cost() {
        let world = world();
        let tools = [(id("tconstruct:pickaxe"), &world.pickaxe)];
        let item_max = |_: &Identifier| 1;
        let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
        let mut tool = build(&ctx, "test:wood", "test:wood");
        hurt(&ctx, &world, &mut tool, 5);
        let inputs = [part("tconstruct:pick_head", "test:wood"), None, None];
        // No casting cost known (the plain lookup): a same-material swap on a damaged tool passes by.
        assert!(matches!(lookup(&ctx, Some(&tool), &inputs).unwrap(), Outcome::NoMatch));
        // Cost 3 against the head's 40 durability repairs 3 * 40 / 3 = 40, covering the damage.
        let cost = |_: &Identifier| 3;
        let raw: Vec<Option<(Identifier, u32)>> = vec![Some((id("tconstruct:pick_head"), 1)), None, None];
        let no_tag = |_: &Identifier, _: &Identifier| false;
        let data = SwapData { part_cost: &cost, recipes: None, raw: &raw, in_tag: &no_tag };
        let Outcome::Crafted(swapped) = lookup_with(&ctx, Some(&tool), &inputs, None, Some(&data)).unwrap() else { panic!("expected a repairing swap") };
        assert_eq!(swapped.result.damage, 0, "the casting cost repaired the tool");
        assert_eq!(swapped.consumed, vec![1, 0, 0]);
        assert_eq!(crate::tool_stack::materials(&swapped.result.custom_data, true).get(0).to_string(), "test:wood", "the material did not change");
    }
    #[test]
    fn fixed_swapping_sets_the_material_and_repairs_by_its_value() {
        let world = world();
        let tools = [(id("tconstruct:pickaxe"), &world.pickaxe)];
        let item_max = |_: &Identifier| 1;
        let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
        let files = vec![(id("tconstruct:tools/armor/slime_skull/swapping/stone"), json!({"type":"tconstruct:fixed_material_swapping","index":0,"ingredient":{"item":"minecraft:skeleton_skull"},"material":"test:stone","max_stack_size":16,"repair_value":6,"tools":{"tag":"test:swappable"}}))];
        let recipes = crate::swapping_recipes::Recipes::load(&files, &Always);
        assert_eq!(recipes.len(), 1, "{:?}", recipes.problems);
        let in_tag = |item: &Identifier, tag: &Identifier| *item == id("tconstruct:pickaxe") && *tag == id("test:swappable");
        let zero = |_: &Identifier| 0;
        let mut tool = build(&ctx, "test:wood", "test:wood");
        hurt(&ctx, &world, &mut tool, 5);
        let inputs = [Some(Input::Other { item: id("minecraft:skeleton_skull") }), None, None];
        let raw: Vec<Option<(Identifier, u32)>> = vec![Some((id("minecraft:skeleton_skull"), 1)), None, None];
        let data = SwapData { part_cost: &zero, recipes: Some(&recipes), raw: &raw, in_tag: &in_tag };
        let Outcome::Crafted(swapped) = lookup_with(&ctx, Some(&tool), &inputs, None, Some(&data)).unwrap() else { panic!("expected a fixed swap") };
        assert_eq!(crate::tool_stack::materials(&swapped.result.custom_data, true).get(0).to_string(), "test:stone");
        assert_eq!(swapped.result.damage, 0, "repair_value 6 * 40 / 3 repairs the 5 damage");
        assert_eq!(swapped.consumed, vec![1, 0, 0]);
        // An unrelated item on the table matches nothing.
        let stray = [Some(Input::Other { item: id("minecraft:dirt") }), None, None];
        assert!(matches!(lookup_with(&ctx, Some(&tool), &stray, None, Some(&data)).unwrap(), Outcome::NoMatch));
        // A fixed material with no stats for the head is a failure, not a pass.
        let bad = vec![(id("tconstruct:tools/armor/slime_skull/swapping/bad"), json!({"type":"tconstruct:fixed_material_swapping","index":0,"ingredient":{"item":"minecraft:skeleton_skull"},"material":"test:unknown","tools":{"tag":"test:swappable"}}))];
        let bad_recipes = crate::swapping_recipes::Recipes::load(&bad, &Always);
        let bad_data = SwapData { part_cost: &zero, recipes: Some(&bad_recipes), raw: &raw, in_tag: &in_tag };
        assert!(matches!(lookup_with(&ctx, Some(&tool), &inputs, None, Some(&bad_data)).unwrap(),
            Outcome::Failure(key) if key == "recipe.tconstruct.part_swapping.invalid_material"));
    }
    #[test]
    fn material_value_swapping_pays_the_cost_and_refunds_the_leftover() {
        let world = world();
        let tools = [(id("tconstruct:pickaxe"), &world.pickaxe)];
        let item_max = |_: &Identifier| 1;
        let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
        let files = vec![(id("tconstruct:tools/armor/travelers/vest"), json!({"type":"tconstruct:material_value_swapping","cost":2,"index":0,"tools":{"tag":"test:swappable"},"material":{"type":"tconstruct:stat_type","stat_type":"tconstruct:head"},"max_stack_size":16}))];
        let recipes = crate::swapping_recipes::Recipes::load(&files, &Always);
        assert_eq!(recipes.len(), 1, "{:?}", recipes.problems);
        let in_tag = |item: &Identifier, tag: &Identifier| *item == id("tconstruct:pickaxe") && *tag == id("test:swappable");
        let zero = |_: &Identifier| 0;
        let material_recipe = crate::material_recipes::Recipe {
            id: id("tconstruct:tools/materials/gem"),
            ingredient: crate::material_recipes::Ingredient::Item(id("test:gem")),
            material: variant("test:stone"), needed: 1, value: 3,
            leftover: Some(crate::material_recipes::Leftover { source: crate::material_recipes::LeftoverSource::Item(id("test:shard")), count: 1 }),
        };
        let input = Some(Input::Material { item: id("test:gem"), material: id("test:stone"), recipe: material_recipe, count: 4 });
        let inputs = [input, None, None];
        let raw: Vec<Option<(Identifier, u32)>> = vec![Some((id("test:gem"), 4)), None, None];
        let data = SwapData { part_cost: &zero, recipes: Some(&recipes), raw: &raw, in_tag: &in_tag };
        let tool = build(&ctx, "test:wood", "test:wood");
        let Outcome::Crafted(swapped) = lookup_with(&ctx, Some(&tool), &inputs, None, Some(&data)).unwrap() else { panic!("expected a value swap") };
        assert_eq!(crate::tool_stack::materials(&swapped.result.custom_data, true).get(0).to_string(), "test:stone");
        // Cost 2 of value 3 per item uses a single item; the unused 1 of 3 comes back as a shard.
        assert_eq!(swapped.consumed, vec![1, 0, 0]);
        assert_eq!(swapped.leftover, vec![(id("test:shard"), 1)]);
        // A predicate the material does not satisfy (no skull stats anywhere) matches nothing.
        let skull = vec![(id("tconstruct:tools/armor/travelers/skull"), json!({"type":"tconstruct:material_value_swapping","cost":2,"index":0,"tools":{"tag":"test:swappable"},"material":{"type":"tconstruct:stat_type","stat_type":"tconstruct:skull"},"max_stack_size":16}))];
        let skull_recipes = crate::swapping_recipes::Recipes::load(&skull, &Always);
        let skull_data = SwapData { part_cost: &zero, recipes: Some(&skull_recipes), raw: &raw, in_tag: &in_tag };
        assert!(matches!(lookup_with(&ctx, Some(&tool), &inputs, None, Some(&skull_data)).unwrap(), Outcome::NoMatch));
    }
    #[test]
    fn tool_swapping_takes_the_material_from_a_sacrifice_and_repairs_from_it() {
        let world = world();
        let tools = [(id("tconstruct:pickaxe"), &world.pickaxe)];
        let item_max = |_: &Identifier| 1;
        let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
        let files = vec![(id("tconstruct:tables/tool_material_swapping"), json!({"type":"tconstruct:tool_material_swapping","tools":{"type":"forge:difference","base":{"tag":"test:multipart"},"subtracted":{"tag":"test:unswappable"}}}))];
        let recipes = crate::swapping_recipes::Recipes::load(&files, &Always);
        assert_eq!(recipes.len(), 1, "{:?}", recipes.problems);
        let in_tag = |item: &Identifier, tag: &Identifier| *item == id("tconstruct:pickaxe") && *tag == id("test:multipart");
        let zero = |_: &Identifier| 0;
        let mut tool = build(&ctx, "test:wood", "test:wood");
        hurt(&ctx, &world, &mut tool, 10);
        let sacrifice = build(&ctx, "test:stone", "test:wood");
        let inputs = [Some(Input::Tool { stack: Box::new(sacrifice.clone()) }), None, None];
        let raw: Vec<Option<(Identifier, u32)>> = vec![Some((id("tconstruct:pickaxe"), 1)), None, None];
        let data = SwapData { part_cost: &zero, recipes: Some(&recipes), raw: &raw, in_tag: &in_tag };
        let Outcome::Crafted(swapped) = lookup_with(&ctx, Some(&tool), &inputs, None, Some(&data)).unwrap() else { panic!("expected a tool swap") };
        assert_eq!(crate::tool_stack::materials(&swapped.result.custom_data, true).get(0).to_string(), "test:stone", "the head took the sacrifice's material");
        assert_eq!(swapped.result.damage, 0, "the sacrifice's remaining durability (plus 5%) repaired the tool");
        assert_eq!(swapped.consumed, vec![1, 0, 0]);
        // A sacrifice carrying upgrades is refused with the no-modifiers failure.
        let mut upgraded = sacrifice.clone();
        let upgrades = crate::tool_stack::upgrades(&upgraded.custom_data).with_modifier(id("test:sharp"), 1).unwrap();
        upgraded.custom_data.insert(crate::tool_stack::TAG_UPGRADES.into(), upgrades.to_nbt());
        let inputs = [Some(Input::Tool { stack: Box::new(upgraded) }), None, None];
        assert!(matches!(lookup_with(&ctx, Some(&tool), &inputs, None, Some(&data)).unwrap(),
            Outcome::Failure(key) if key == "recipe.tconstruct.part_swapping.no_modifiers"));
        // A different item in the slot is no sacrifice at all.
        let stick = ItemSlot::new(id("minecraft:stick"), 1);
        let inputs = [Some(Input::Tool { stack: Box::new(stick) }), None, None];
        assert!(matches!(lookup_with(&ctx, Some(&tool), &inputs, None, Some(&data)).unwrap(), Outcome::NoMatch));
    }
}
