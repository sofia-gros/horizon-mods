//! ToolStack.createTool and ToolBuildingRecipe matching/result from SlimeKnights (MIT; see LICENSE).
//! Pure over item identifiers and material variants; resolving real stacks to those, shrinking
//! inputs, ToolStack.tryValidate and the CRAFT_COUNT modifier chain (identity until a modifier
//! defines it) belong to the caller.
use crate::{
    tool_materials::{MaterialList, MaterialVariant},
    tool_stack::{rebuild_stats, set_materials_raw, Environment},
};
use pluto_lib::{Identifier, Nbt, NbtCompound};

const NO_COUNT: &str = "recipe.tconstruct.tool_build.no_count";

/// ToolStack.createTool: a fresh compound with materials, then a rebuild. If the environment is
/// not ready the rebuild is skipped and the tool stays uninitialized, exactly as in the original.
/// `empty_sentinel` is MaterialNBT.EMPTY itself (removes the key) as opposed to an equal but
/// distinct empty list built by a recipe (which saves an empty list).
pub(crate) fn create_tool(env: &Environment, materials: &MaterialList, empty_sentinel: bool) -> Result<NbtCompound, String> {
    let mut nbt = NbtCompound::new();
    if materials.as_slice().is_empty() && !empty_sentinel {
        nbt.insert(crate::tool_stack::TAG_MATERIALS.into(), Nbt::List(Vec::new()));
    } else {
        set_materials_raw(&mut nbt, materials);
    }
    rebuild_stats(&mut nbt, env)?;
    Ok(nbt)
}

/// CraftCountModifierHook.maxStackSize with no modifier hooks running (the base the chain multiplies):
/// single-stack items stay at 1. The full hook over a built tool is tool_stack::max_stack_size.
pub(crate) fn craft_count(item_max: i32, count: f32) -> i32 {
    if item_max == 1 { return 1; }
    if count <= 0.0 { return 0; }
    (count.min(item_max as f32)) as i32
}

/// One recipe ingredient slot; `None` entries in the inventory are empty slots.
pub(crate) type Ingredient<'a> = &'a dyn Fn(Option<&Identifier>) -> bool;

/// ToolBuildingRecipe.matches. `inputs` holds the item of every input slot (None = empty) and
/// `tool_slot_empty` says the tinkerable slot is empty. Ingredients past the list match only empty.
pub(crate) fn matches(tool_slot_empty: bool, inputs: &[Option<Identifier>], parts: &[Identifier], extra: &[Ingredient]) -> bool {
    if !tool_slot_empty { return false; }
    let required = parts.len() + extra.len();
    if required == 0 || required > inputs.len() { return false; }
    // A single non-part input may sit in any slot, but must be the only non-empty one.
    if required == 1 && parts.is_empty() {
        let mut found = false;
        for input in inputs.iter().flatten() {
            if found || !extra[0](Some(input)) { return false; }
            found = true;
        }
        return found;
    }
    for (slot, part) in parts.iter().enumerate() {
        if inputs[slot].as_ref() != Some(part) { return false; }
    }
    for (slot, input) in inputs.iter().enumerate().skip(parts.len()) {
        let accepted = match extra.get(slot - parts.len()) { Some(ingredient) => ingredient(input.as_ref()), None => input.is_none() };
        if !accepted { return false; }
    }
    true
}

#[derive(Debug)]
pub(crate) enum Built {
    Failure(String),
    Success { tool: NbtCompound, count: i32 },
}

/// ToolBuildingRecipe.getValidatedResult. `part_materials` is the material of each part input in
/// slot order; `extra_materials` are applied after the parts while stat slots remain.
pub(crate) fn build(
    env: &Environment, part_materials: &[MaterialVariant], extra_materials: &[MaterialVariant],
    parts: usize, output_count: i32, item_max: i32,
) -> Result<Built, String> {
    let slots = env.definition.and_then(|definition| definition.material_stat_types()).map_or(0, <[_]>::len);
    let mut materials = Vec::new();
    if slots > 0 {
        let count = parts.min(slots);
        materials.extend(part_materials.iter().take(count).cloned());
        let extras = slots.saturating_sub(parts).min(extra_materials.len());
        materials.extend(extra_materials.iter().take(extras).cloned());
    }
    let list = MaterialList::from_variants(materials);
    let tool = create_tool(env, &list, false)?;
    // Only recipes with real parts consult the craft-count chain (over the tool just built).
    let count = if parts > 0 && item_max > 1 {
        let count = crate::tool_stack::max_stack_size(&tool, env, item_max, output_count);
        if count <= 0 { return Ok(Built::Failure(NO_COUNT.to_owned())); }
        count
    } else { output_count.min(item_max) };
    // Traits may reject each other or overspend slots: a tool that fails validation is not produced.
    if let Some(error) = crate::tool_stack::try_validate(&tool, env) { return Ok(Built::Failure(error)); }
    Ok(Built::Success { tool, count })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{materials::{Definitions as Materials, Material, Rarity}, modifiers::Registry, tool_definitions::Data, tool_stack::{damage, durability, is_initialized}};
    use serde_json::json;
    struct Always;
    impl crate::materials::Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &serde_json::Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn variant(text: &str) -> MaterialVariant { MaterialVariant::parse(text).unwrap() }
    fn world() -> (Data, Materials, Registry) {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:material_stats","stat_types":["tconstruct:head","tconstruct:handle","tconstruct:binding"]}
        ]})).unwrap();
        let mut materials = Materials::default();
        materials.stats = crate::material_stats::load(&[pluto_lib::JsonResourceStack { id: id("test:wood"), layers: vec![
            Ok(json!({"stats":{"tconstruct:head":{"durability":30},"tconstruct:handle":{"durability":0.5}}}))] }], &mut materials.problems);
        materials.materials.insert(id("test:wood"), Material { id: id("test:wood"), tier: 1, sort_order: 1, rarity: Rarity::Common, craftable: true, hidden: false });
        materials.server_loaded = true;
        let mut registry = Registry::default();
        registry.reload(&[], &Always);
        (data, materials, registry)
    }
    #[test]
    fn parts_fill_materials_then_overrides_and_the_tool_rebuilds() {
        let (data, materials, registry) = world();
        let env = Environment { definition: Some(&data), has_materials: true, materials: &materials, modifiers: &registry, item_tags_loaded: true, item: None, item_tags: None };
        // Two parts and one override fills the three stat slots; extra overrides are dropped.
        let Built::Success { tool, count } = build(&env, &[variant("test:wood"), variant("test:wood")], &[variant("test:wood"), variant("test:stone")], 2, 1, 1).unwrap()
            else { panic!() };
        assert_eq!(count, 1);
        assert!(is_initialized(&tool));
        assert_eq!(crate::tool_stack::materials(&tool, true).as_slice().len(), 3);
        assert_eq!((damage(&tool), durability(&tool) > 1), (0, true));
        // Not ready (item tags missing): the tool is created but never initialized.
        let unready = Environment { item_tags_loaded: false, ..Environment { definition: Some(&data), has_materials: true, materials: &materials, modifiers: &registry, item_tags_loaded: true, item: None, item_tags: None } };
        let Built::Success { tool, .. } = build(&unready, &[variant("test:wood"), variant("test:wood")], &[], 2, 1, 1).unwrap() else { panic!() };
        assert!(!is_initialized(&tool));
    }
    #[test]
    fn count_follows_the_craft_count_chain_only_for_part_recipes() {
        let (data, materials, registry) = world();
        let env = Environment { definition: Some(&data), has_materials: true, materials: &materials, modifiers: &registry, item_tags_loaded: true, item: None, item_tags: None };
        let count = |parts, output, item_max| match build(&env, &[variant("test:wood"), variant("test:wood")], &[], parts, output, item_max).unwrap() {
            Built::Success { count, .. } => count, Built::Failure(_) => -1 };
        assert_eq!(count(2, 4, 64), 4);
        assert_eq!(count(2, 100, 64), 64, "capped by the item's stack size");
        assert_eq!(count(2, 4, 1), 1, "single-stack items never multiply");
        assert_eq!(count(0, 100, 16), 16, "no parts skips the chain and takes the plain minimum");
        assert_eq!(craft_count(64, 0.0), 0);
    }
    #[test]
    fn matching_requires_empty_tool_slot_ordered_parts_and_exact_extras() {
        let head = id("test:head");
        let handle = id("test:handle");
        let any = |slot: Option<&Identifier>| slot.is_some_and(|item| item.path() == "stick");
        let stick = id("test:stick");
        let inputs = |items: &[Option<&Identifier>]| items.iter().map(|item| item.cloned()).collect::<Vec<_>>();
        assert!(matches(true, &inputs(&[Some(&head), Some(&handle), None]), &[head.clone(), handle.clone()], &[]));
        assert!(!matches(false, &inputs(&[Some(&head), Some(&handle)]), &[head.clone(), handle.clone()], &[]), "tool slot must be empty");
        assert!(!matches(true, &inputs(&[Some(&handle), Some(&head)]), &[head.clone(), handle.clone()], &[]), "parts are positional");
        assert!(!matches(true, &inputs(&[Some(&head), Some(&handle), Some(&stick)]), &[head.clone(), handle.clone()], &[]), "no stray extra input");
        assert!(matches(true, &inputs(&[Some(&head), Some(&handle), Some(&stick)]), &[head.clone(), handle.clone()], &[&any]));
        assert!(!matches(true, &inputs(&[Some(&head)]), &[head.clone(), handle.clone()], &[]), "too few slots");
        assert!(!matches(true, &inputs(&[Some(&head)]), &[], &[]), "a recipe needs at least one input");
        // A lone non-part ingredient may sit in any slot, but it must be the only item.
        assert!(matches(true, &inputs(&[None, Some(&stick), None]), &[], &[&any]));
        assert!(!matches(true, &inputs(&[Some(&stick), Some(&stick)]), &[], &[&any]));
        assert!(!matches(true, &inputs(&[None, None]), &[], &[&any]));
    }
    #[test]
    fn create_tool_distinguishes_the_empty_sentinel_from_an_empty_list() {
        let (data, materials, registry) = world();
        let env = Environment { definition: Some(&data), has_materials: false, materials: &materials, modifiers: &registry, item_tags_loaded: true, item: None, item_tags: None };
        let sentinel = create_tool(&env, &MaterialList::default(), true).unwrap();
        assert!(!sentinel.contains_key(crate::tool_stack::TAG_MATERIALS));
        let list = create_tool(&env, &MaterialList::default(), false).unwrap();
        assert_eq!(list[crate::tool_stack::TAG_MATERIALS], Nbt::List(Vec::new()));
    }
}
