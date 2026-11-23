// Tinker station menu bridge: staged stacks -> resolved inputs -> ToolStation outcome -> consumption.
// Ported from TinkerStationBlockEntity.craftResult / updateInputs (SlimeKnights, MIT; see ../../../LICENSE).
// Java revision: f4e77d43f39ba1047bf93ae816b7e1beaab514c2.

use pluto_lib::{ItemSlot, MenuAccess};
use super::{input_consumption::shrink_menu_input, lazy_result::LazyResultContainer};
use crate::{material_recipes::Recipes, tool_materials::part_material, tool_parts, tool_station::{self, Context, Crafted, Input, Outcome}};
use pluto_lib::Identifier;

const REPAIR_KIT: &str = "tconstruct:repair_kit";

/// What turns a stack into a station input: the repair kit amount and the material recipes with
/// the item-tag lookup their ingredients use.
pub(crate) struct Resolver<'a> {
    pub kit_amount: f32,
    pub recipes: Option<&'a Recipes>,
    pub modifier_recipes: Option<&'a crate::modifier_recipes::Recipes>,
    pub overslime_recipes: Option<&'a crate::overslime::Recipes>,
    pub modifier_repair_recipes: Option<&'a crate::modifier_repair::Recipes>,
    pub damaging_recipes: Option<&'a crate::damaging::Recipes>,
    pub dyeing_recipes: Option<&'a crate::dyeing::Recipes>,
    pub banner_recipes: Option<&'a crate::banner_modifier::Recipes>,
    pub trim_recipes: Option<&'a crate::armor_trim::Recipes>,
    pub in_tag: &'a dyn Fn(&Identifier, &Identifier) -> bool,
}

/// Resolves one non-empty stack the way the station recipes see it: a tool part, a repair kit, an
/// item a material recipe maps to a material, or anything else.
pub(crate) fn resolve_input(stack: &ItemSlot, resolver: &Resolver) -> Input {
    let kit_amount = resolver.kit_amount;
    if tool_parts::registry().contains_key(&stack.item) {
        return Input::Part { item: stack.item.clone(), material: part_material(&stack.custom_data) };
    }
    if stack.item.to_string() == REPAIR_KIT {
        // A kit with no material is MaterialId.UNKNOWN: it cannot take part in any recipe.
        if let Some(material) = part_material(&stack.custom_data).material_id().filter(|id| id.to_string() != "tconstruct:unknown") {
            return Input::Kit { material, kit_amount, count: i32::try_from(stack.count).unwrap_or(i32::MAX) };
        }
    }
    let instrument = crate::material_recipes::instrument_of(stack);
    if let Some(recipe) = resolver.recipes.and_then(|recipes| recipes.find_facts(&stack.item, resolver.in_tag, &crate::material_recipes::StackFacts { instrument: instrument.as_ref(), ..Default::default() })) {
        if let Some(material) = recipe.material.material_id() {
            return Input::Material { item: stack.item.clone(), material, recipe: recipe.clone(), count: i32::try_from(stack.count).unwrap_or(i32::MAX) };
        }
    }
    Input::Other { item: stack.item.clone() }
}

/// The tinkerable stack (slot 0) and each input (slots 1..=input_count; None when empty).
pub(crate) fn read(menu: &dyn MenuAccess, input_count: usize, resolver: &Resolver) -> (Option<ItemSlot>, Vec<Option<Input>>) {
    let tool = menu.stack(0).filter(|stack| stack.count > 0);
    let inputs = (1..=input_count).map(|slot| menu.stack(slot).filter(|stack| stack.count > 0).map(|stack| resolve_input(&stack, resolver))).collect();
    (tool, inputs)
}

/// What the station would produce for the staged menu, with no side effects.
pub(crate) fn preview(menu: &dyn MenuAccess, ctx: &Context, input_count: usize, resolver: &Resolver) -> Result<Outcome, String> {
    let (tool, mut inputs) = read(menu, input_count, resolver);
    // A tinkerable tool stack in an input slot is the sacrifice of tool material swapping;
    // resolve_input cannot know the tool definitions, so it is recognized here.
    for (index, input) in inputs.iter_mut().enumerate() {
        let is_tool = matches!(input, Some(Input::Other { item }) if ctx.tools.iter().any(|(tool_item, _)| tool_item == item));
        if is_tool {
            if let Some(stack) = menu.stack(index + 1).filter(|stack| stack.count > 0) {
                *input = Some(Input::Tool { stack: Box::new(stack) });
            }
        }
    }
    let raw: Vec<Option<(Identifier, u32)>> = (1..=input_count)
        .map(|slot| menu.stack(slot).filter(|stack| stack.count > 0).map(|stack| (stack.item.clone(), stack.count))).collect();
    // A modifier crystal carries its modifier in the stack's data, which the plain item and count above do not.
    let stacks: Vec<Option<(Identifier, u32, Option<Identifier>)>> = (1..=input_count).map(|slot| menu.stack(slot).filter(|stack| stack.count > 0).map(|stack| {
        let modifier = match stack.custom_data.get(crate::modifier_recipes::CRYSTAL_KEY) { Some(pluto_lib::Nbt::String(text)) => Identifier::parse(text).ok(), _ => None };
        (stack.item.clone(), stack.count, modifier)
    })).collect();
    let crystal = crate::modifier_recipes::lone_crystal(&stacks);
    // The full input stacks: the banner recipe reads the banner stack's pattern data.
    let full_stacks: Vec<Option<ItemSlot>> = (1..=input_count).map(|slot| menu.stack(slot).filter(|stack| stack.count > 0)).collect();
    let modifiers = resolver.modifier_recipes.map(|recipes| tool_station::ModifierRecipes { recipes, crystal, overslime: resolver.overslime_recipes, modifier_repairs: resolver.modifier_repair_recipes, damaging: resolver.damaging_recipes, dyeing: resolver.dyeing_recipes, banner: resolver.banner_recipes, trim: resolver.trim_recipes, raw: &raw, stacks: &full_stacks, in_tag: resolver.in_tag });
    // The part swap's repair and the material swapping recipes: the casting lookup lives
    // with the smeltery recipes, the swapping recipes in their own registry.
    let casting = crate::common::smeltery_data().and_then(|data| data.read().ok().map(|data| data.recipes.material_casting.clone()));
    let part_cost = |item: &Identifier| casting.as_ref().map_or(0, |casting| casting.item_cost(item));
    let swapping = crate::common::swapping_recipes().and_then(|recipes| recipes.read().ok().map(|recipes| recipes.clone()));
    // The repair gate disables the station's material repair only once a reload has run
    // without the fieldless tinker_station_repair recipe; before any reload (or where the
    // registry was never initialized) repair runs as the station's own, which is the
    // original's behaviour with its shipped recipe present.
    let repair_gate = crate::common::repair_gate().and_then(|gate| gate.read().ok());
    let part_swapping = crate::common::part_swapping().and_then(|recipes| recipes.read().ok());
    let building_recipes = crate::common::building_recipes().and_then(|recipes| recipes.read().ok());
    // The tipped transform reads the first non-empty input stack's potion.
    let potion = (1..=input_count).filter_map(|slot| menu.stack(slot)).find(|stack| stack.count > 0)
        .and_then(|stack| match stack.custom_data.get("Potion") { Some(pluto_lib::Nbt::String(text)) => Some(text.clone()), _ => None });
    let building_data = building_recipes.as_deref().filter(|recipes| recipes.loaded)
        .map(|recipes| tool_station::BuildingData { recipes, potion: potion.as_deref() });
    let swap_data = tool_station::SwapData { part_cost: &part_cost, recipes: swapping.as_ref(), raw: &raw, in_tag: resolver.in_tag,
        repair_present: repair_gate.as_ref().map_or(true, |gate| !gate.loaded || gate.present),
        part_swapping: part_swapping.as_deref().filter(|recipes| recipes.loaded), building: building_data };
    tool_station::lookup_with(ctx, tool.as_ref(), &inputs, modifiers.as_ref(), Some(&swap_data))
}

/// craftResult: computes the outcome, consumes the inputs and the tool, and hands back the result.
/// `Ok(None)` means nothing was crafted (no match or a recipe failure), with nothing changed.
pub(crate) fn take(
    menu: &mut dyn MenuAccess, lazy: &mut LazyResultContainer, ctx: &Context, input_count: usize, resolver: &Resolver,
) -> Result<Option<ItemSlot>, String> {
    let Outcome::Crafted(Crafted { result, consumed, tool_shrink, leftover }) = preview(menu, ctx, input_count, resolver)? else { return Ok(None); };
    for (index, amount) in consumed.iter().enumerate() {
        let amount = u32::try_from(*amount).unwrap_or(0);
        if amount == 0 { continue; }
        let container = menu.stack(index + 1).and_then(|stack| menu.crafting_remainder(&stack));
        let index = i32::try_from(index).map_err(|_| "station input index overflow")?;
        if let Some(extra) = shrink_menu_input(menu, lazy, input_count, index, amount, container).map_err(str::to_owned)? {
            menu.give_stack(extra);
        }
    }
    for (item, count) in leftover { menu.give_stack(ItemSlot::new(item, count)); }
    shrink_tool_slot(menu, lazy, tool_shrink)?;
    Ok(Some(result))
}

/// Takes `shrink` tools from slot 0; reaching zero empties it. A count of 0 leaves it alone.
fn shrink_tool_slot(menu: &mut dyn MenuAccess, lazy: &mut LazyResultContainer, shrink: i32) -> Result<(), String> {
    let shrink = u32::try_from(shrink).unwrap_or(0);
    if shrink == 0 { return Ok(()); }
    let Some(mut tool) = menu.stack(0).filter(|stack| stack.count > 0) else { return Ok(()); };
    let next = if tool.count <= shrink { None } else { tool.count -= shrink; Some(tool) };
    if !menu.set_stack(0, next) { return Err("station tool write refused".into()); }
    lazy.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{materials::{Definitions as Materials, Material, Rarity}, modifiers::Registry, tool_definitions::Data, tool_materials::set_part_material};
    use pluto_lib::Identifier;
    use serde_json::json;

    struct Always;
    impl crate::materials::Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &serde_json::Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }
    struct Menu { slots: Vec<Option<ItemSlot>>, given: Vec<ItemSlot> }
    impl MenuAccess for Menu {
        fn slot_count(&self) -> usize { self.slots.len() }
        fn stack(&self, slot: usize) -> Option<ItemSlot> { self.slots[slot].clone() }
        fn set_stack(&mut self, slot: usize, stack: Option<ItemSlot>) -> bool { self.slots[slot] = stack; true }
        fn swap_stack(&self, _: i32) -> Option<ItemSlot> { panic!("unexpected swap") }
        fn set_swap_stack(&mut self, _: i32, _: Option<ItemSlot>) -> bool { panic!("unexpected swap") }
        fn carried(&self) -> Option<ItemSlot> { panic!("unexpected cursor") }
        fn set_carried(&mut self, _: Option<ItemSlot>) { panic!("unexpected cursor") }
        fn same_item_and_components(&self, _: &ItemSlot, _: &ItemSlot) -> bool { panic!("unexpected merge") }
        fn max_stack_count(&self, _: &ItemSlot) -> u32 { panic!("unexpected limit") }
        fn move_stack(&mut self, _: &mut ItemSlot, _: usize, _: usize, _: bool) -> bool { panic!("unexpected transfer") }
        fn crafting_remainder(&self, _: &ItemSlot) -> Option<ItemSlot> { None }
        fn give_stack(&mut self, stack: ItemSlot) { self.given.push(stack); }
        fn drop_stack(&mut self, _: &ItemSlot) { panic!("unexpected drop") }
    }
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn part(item: &str, material: &str, count: u32) -> ItemSlot {
        let mut stack = ItemSlot::new(id(item), count);
        set_part_material(&mut stack, &crate::tool_materials::MaterialVariant::parse(material).unwrap());
        stack
    }
    #[test]
    fn building_takes_one_of_each_part_and_a_taken_tool_can_be_repaired_with_a_kit() {
        let pickaxe = Data::decode(&json!({"modules":[
            {"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle"]},
            {"type":"tconstruct:is_effective","predicate_type":"mantle:tag","tag":"minecraft:mineable/pickaxe"}
        ]})).unwrap();
        let mut materials = Materials::default();
        materials.stats = crate::material_stats::load(&[pluto_lib::JsonResourceStack { id: id("test:wood"), layers: vec![
            Ok(json!({"stats":{"tconstruct:head":{"durability":40,"mining_speed":3},"tconstruct:handle":{"durability":0.5}}}))] }], &mut materials.problems);
        materials.materials.insert(id("test:wood"), Material { id: id("test:wood"), tier: 1, sort_order: 1, rarity: Rarity::Common, craftable: true, hidden: false });
        materials.server_loaded = true;
        let mut registry = Registry::default();
        registry.reload(&[], &Always);
        let tools = [(id("tconstruct:pickaxe"), &pickaxe)];
        let item_max = |_: &Identifier| 1;
        let ctx = Context { materials: &materials, modifiers: &registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
        let resolver = Resolver { kit_amount: 2.0, recipes: None, modifier_recipes: None, overslime_recipes: None, modifier_repair_recipes: None, damaging_recipes: None, dyeing_recipes: None, banner_recipes: None, trim_recipes: None, in_tag: &|_, _| false };
        let mut lazy = LazyResultContainer::default();
        lazy.result(|| Some(ItemSlot::new(id("test:stale"), 1)));
        // Slots: tool, 4 inputs, result placeholder.
        let mut menu = Menu { slots: vec![None, Some(part("tconstruct:pick_head", "test:wood", 2)), Some(part("tconstruct:tool_handle", "test:wood", 1)), None, None, None], given: vec![] };
        assert!(matches!(preview(&menu, &ctx, 4, &resolver).unwrap(), Outcome::Crafted(_)));
        let built = take(&mut menu, &mut lazy, &ctx, 4, &resolver).unwrap().expect("a tool");
        assert_eq!(built.item.to_string(), "tconstruct:pickaxe");
        assert_eq!(menu.slots[1].as_ref().unwrap().count, 1, "one head left of two");
        assert!(menu.slots[2].is_none(), "the single handle was consumed");
        assert!(lazy.result(|| None).is_none(), "consumption invalidated the cached preview");
        // Nothing matches an empty table or stray items, and nothing is consumed then.
        let mut empty = Menu { slots: vec![None; 6], given: vec![] };
        assert!(take(&mut empty, &mut lazy, &ctx, 4, &resolver).unwrap().is_none());
        // Damage the tool and repair it with a wood repair kit.
        let mut tool = built.clone();
        crate::tool_slot::damage_slot(&mut tool, &Environment4::of(&ctx, &pickaxe), 10, true, true, None, &mut || 1.0).unwrap();
        let mut kit = ItemSlot::new(id("tconstruct:repair_kit"), 3);
        set_part_material(&mut kit, &crate::tool_materials::MaterialVariant::parse("test:wood").unwrap());
        let mut menu = Menu { slots: vec![Some(tool), Some(kit), None, None, None, None], given: vec![] };
        let repaired = take(&mut menu, &mut lazy, &ctx, 4, &resolver).unwrap().expect("a repaired tool");
        assert_eq!(repaired.damage, 0);
        assert!(menu.slots[0].is_none(), "the damaged tool slot is consumed (shrink by one)");
        assert!(menu.slots[1].as_ref().map_or(true, |kit| kit.count < 3), "some kit was used");
        // A material recipe maps a plain item to a material: three planks' worth repairs the head part.
        let recipes = crate::material_recipes::Recipes::load(&[(id("tconstruct:tools/materials/plank"), json!({"type":"tconstruct:material",
            "ingredient":{"item":"test:plank"},"material":"test:wood","needed":1,"value":1}))], &Always);
        assert_eq!(recipes.len(), 1);
        let with_recipes = Resolver { kit_amount: 2.0, recipes: Some(&recipes), modifier_recipes: None, overslime_recipes: None, modifier_repair_recipes: None, damaging_recipes: None, dyeing_recipes: None, banner_recipes: None, trim_recipes: None, in_tag: &|_, _| false };
        let plank = ItemSlot::new(id("test:plank"), 5);
        assert!(matches!(resolve_input(&plank, &with_recipes), Input::Material { count: 5, recipe, .. } if recipe.value == 1 && recipe.needed == 1));
        assert!(matches!(resolve_input(&plank, &resolver), Input::Other { .. }), "without recipes the plank is just an item");
        let mut hurt = built.clone();
        crate::tool_slot::damage_slot(&mut hurt, &Environment4::of(&ctx, &pickaxe), 10, true, true, None, &mut || 1.0).unwrap();
        let mut menu = Menu { slots: vec![Some(hurt), Some(plank), None, None, None, None], given: vec![] };
        let fixed = take(&mut menu, &mut lazy, &ctx, 4, &with_recipes).unwrap().expect("repaired with planks");
        assert_eq!(fixed.damage, 0);
        // 40 durability * value 1 / 3 ingots / needed 1 = 13.3 per plank, so one plank covers 10 damage.
        assert_eq!(menu.slots[1].as_ref().unwrap().count, 4, "exactly one plank was consumed");
        // A kit with no material resolves to a stray item.
        assert!(matches!(resolve_input(&ItemSlot::new(id("tconstruct:repair_kit"), 1), &resolver), Input::Other { .. }));
    }
    struct Environment4;
    impl Environment4 {
        fn of<'a>(ctx: &'a Context, definition: &'a Data) -> crate::tool_stack::Environment<'a> {
            crate::tool_stack::Environment { definition: Some(definition), has_materials: definition.material_stat_types().is_some(),
                materials: ctx.materials, modifiers: ctx.modifiers, item_tags_loaded: ctx.item_tags_loaded, item: None, item_tags: None }
        }
    }
}
