//! MaterialSwappingRecipe.swapMaterial from SlimeKnights (MIT; see LICENSE).
//! Computes the swapped tool copy. Consuming the part item and extra requirements are the
//! caller's job; ToolStack.tryValidate, ModifierRemovalHook and CraftCountModifierHook (over
//! the original for the shrink, over the copy for the output size) run here.
use crate::{
    tool_damage, tool_materials::MaterialVariant,
    tool_stack::{damage, materials, rebuild_stats, set_materials_raw, Environment},
};
use pluto_lib::NbtCompound;

const INGOTS_PER_REPAIR: f32 = 3.0;
const TOO_FEW_INPUTS: &str = "recipe.tconstruct.part_swapping.too_few_inputs";
const TOO_FEW_PARTS: &str = "recipe.tconstruct.part_swapping.too_few_parts";

#[derive(Debug)]
pub(crate) enum Outcome {
    /// RecipeResult.pass: let other recipes try.
    Pass,
    /// RecipeResult.failure with a translation key.
    Failure(String),
    Success { tool: NbtCompound, size: i32, shrink: i32 },
}

/// `part_value` is the part's item cost (MaterialCastingLookup.getItemCost); `max_stack` is the
/// recipe's max_stack_size and `item_max` the result item's own maximum, which the craft-count
/// hook caps at. `may_repair` forces a swap on a damaged tool even without a change.
pub(crate) fn swap_material(
    original: &NbtCompound, stack_count: i32, env: &Environment, new_material: &MaterialVariant,
    index: usize, part_value: i32, mut may_repair: bool, max_stack: i32, item_max: i32,
) -> Result<Outcome, String> {
    let shrink = crate::tool_stack::max_stack_size(original, env, item_max, max_stack);
    if shrink <= 0 { return Ok(Outcome::Failure(TOO_FEW_INPUTS.to_owned())); }
    let Some(definition) = env.definition else { return Ok(Outcome::Pass); };
    let stat_types = definition.material_stat_types().unwrap_or(&[]);
    if index >= stat_types.len() { return Ok(Outcome::Failure(TOO_FEW_PARTS.to_owned())); }

    let current = materials(original, env.has_materials);
    // A variant-less ID and "#default" are the same variant; anything else is a change.
    let did_change = current.get(index as i32).normalized() != new_material.normalized();
    let mut repair_durability = 0.0_f32;
    if part_value > 0 {
        let durability = new_material.material_id()
            .map(|id| crate::tool_definitions::repair_durability_of(env.materials, &id, &stat_types[index].0)).unwrap_or(0);
        repair_durability = part_value as f32 * durability as f32;
        may_repair |= repair_durability > 0.0;
    }
    if !did_change && (damage(original) == 0 || !may_repair) { return Ok(Outcome::Pass); }

    let mut copy = original.clone();
    if did_change {
        let replaced = current.replaced(index as i32, new_material.clone())?;
        set_materials_raw(&mut copy, &replaced);
        rebuild_stats(&mut copy, env)?;
    }
    // Three ingots repair fully; the part's own cost already counts toward the amount.
    if repair_durability > 0.0 {
        repair_durability /= INGOTS_PER_REPAIR;
        if repair_durability > 0.0 { repair_durability = crate::tool_stack::repair_factor(&copy, env, repair_durability); }
        if repair_durability > 0.0 { tool_damage::repair(&mut copy, repair_durability as i32); }
    }
    // ToolStack.tryValidate on the swapped copy (slot counts and modifier requirements).
    if let Some(error) = crate::tool_stack::try_validate(&copy, env) { return Ok(Outcome::Failure(error)); }
    // ModifierRemovalHook.onRemoved: a trait the new material dropped clears its persistent data.
    crate::tool_stack::on_removed(original, &mut copy, env);
    let output_max = crate::tool_stack::max_stack_size(&copy, env, item_max, max_stack);
    let size = (stack_count.wrapping_mul(output_max) / shrink).min(output_max);
    if size <= 0 { return Ok(Outcome::Failure(TOO_FEW_INPUTS.to_owned())); }
    Ok(Outcome::Success { tool: copy, size, shrink })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{materials::{Definitions as Materials, Material, Rarity}, modifiers::Registry, tool_definitions::Data, tool_stack::{is_initialized, TAG_MATERIALS}};
    use pluto_lib::{Identifier, JsonResourceStack, Nbt};
    use serde_json::json;
    struct Always;
    impl crate::materials::Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &serde_json::Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn world() -> (Data, Materials, Registry) {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:material_stats","stat_types":["tconstruct:head","tconstruct:handle"]}
        ]})).unwrap();
        let mut materials = Materials::default();
        materials.stats = crate::material_stats::load(&[
            JsonResourceStack { id: id("test:wood"), layers: vec![Ok(json!({"stats":{"tconstruct:head":{"durability":30},"tconstruct:handle":{"durability":0.5}}}))] },
            JsonResourceStack { id: id("test:stone"), layers: vec![Ok(json!({"stats":{"tconstruct:head":{"durability":90},"tconstruct:handle":{"durability":0.5}}}))] },
        ], &mut materials.problems);
        for name in ["test:wood", "test:stone"] {
            materials.materials.insert(id(name), Material { id: id(name), tier: 1, sort_order: 1, rarity: Rarity::Common, craftable: true, hidden: false });
        }
        materials.server_loaded = true;
        let mut registry = Registry::default();
        registry.reload(&[], &Always);
        (data, materials, registry)
    }
    fn tool(env: &Environment, names: &[&str]) -> NbtCompound {
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_MATERIALS.into(), Nbt::List(names.iter().map(|n| Nbt::String((*n).into())).collect()));
        assert!(rebuild_stats(&mut nbt, env).unwrap());
        nbt
    }
    #[test]
    fn swapping_a_head_changes_materials_rebuilds_stats_and_repairs_by_part_cost() {
        let (data, materials, registry) = world();
        let env = Environment { definition: Some(&data), has_materials: true, materials: &materials, modifiers: &registry, item_tags_loaded: true, item: None, item_tags: None };
        let mut original = tool(&env, &["test:wood", "test:wood"]);
        let before = crate::tool_stack::durability(&original);
        tool_damage::damage_tool(&mut original, 10, true, true);
        // Stone head (90 durability) costing 3 items repairs 3*90/3 = 90, capped by the 10 damage.
        let Outcome::Success { tool: swapped, size, .. } = swap_material(&original, 1, &env, &MaterialVariant::parse("test:stone").unwrap(), 0, 3, false, 1, 64).unwrap()
            else { panic!("expected success") };
        assert_eq!(size, 1);
        assert!(is_initialized(&swapped));
        assert_eq!(materials_of(&swapped, &env), vec!["test:stone", "test:wood"]);
        assert!(crate::tool_stack::durability(&swapped) > before, "stone head raises durability");
        assert_eq!(damage(&swapped), 0);
        assert_eq!(damage(&original), 10, "the input tool is untouched");
    }
    fn materials_of(nbt: &NbtCompound, env: &Environment) -> Vec<String> {
        materials(nbt, env.has_materials).as_slice().iter().map(|m| m.to_string()).collect()
    }
    #[test]
    fn unchanged_undamaged_or_unrepairing_swaps_pass_and_bad_indices_fail() {
        let (data, materials, registry) = world();
        let env = Environment { definition: Some(&data), has_materials: true, materials: &materials, modifiers: &registry, item_tags_loaded: true, item: None, item_tags: None };
        let mut nbt = tool(&env, &["test:wood", "test:stone"]);
        let wood = MaterialVariant::parse("test:wood").unwrap();
        // Same variant ("#default" is the same) on an undamaged tool does nothing.
        assert!(matches!(swap_material(&nbt, 1, &env, &MaterialVariant::parse("test:wood#default").unwrap(), 0, 3, false, 1, 64).unwrap(), Outcome::Pass));
        // Damaged and the head repairs: a same-material swap still repairs.
        tool_damage::damage_tool(&mut nbt, 12, true, true);
        let Outcome::Success { tool: repaired, .. } = swap_material(&nbt, 1, &env, &wood, 0, 3, false, 1, 64).unwrap() else { panic!() };
        assert_eq!(damage(&repaired), 0);
        // The handle already holds stone, is not repairable and nothing forces it: pass.
        assert!(matches!(swap_material(&nbt, 1, &env, &MaterialVariant::parse("test:stone").unwrap(), 1, 3, false, 1, 64).unwrap(), Outcome::Pass));
        // Forcing the repair (manual repair logic) swaps nothing but still heals.
        let Outcome::Success { tool: forced, .. } = swap_material(&nbt, 1, &env, &MaterialVariant::parse("test:stone").unwrap(), 1, 3, true, 1, 64).unwrap() else { panic!() };
        assert_eq!(damage(&forced), 12 - 0, "a handle has no repair durability, so nothing heals");
        assert!(matches!(swap_material(&nbt, 1, &env, &wood, 2, 1, false, 1, 64).unwrap(), Outcome::Failure(key) if key.ends_with("too_few_parts")));
        assert!(matches!(swap_material(&nbt, 1, &env, &wood, 0, 1, false, 0, 64).unwrap(), Outcome::Failure(key) if key.ends_with("too_few_inputs")));
    }
    #[test]
    fn result_size_scales_with_the_input_stack() {
        let (data, materials, registry) = world();
        let env = Environment { definition: Some(&data), has_materials: true, materials: &materials, modifiers: &registry, item_tags_loaded: true, item: None, item_tags: None };
        let nbt = tool(&env, &["test:wood", "test:wood"]);
        let stone = MaterialVariant::parse("test:stone").unwrap();
        // max_stack 16 with only 8 tools in the stack gives 8; a full stack is capped at 16.
        let size = |count| match swap_material(&nbt, count, &env, &stone, 1, 0, false, 16, 64).unwrap() { Outcome::Success { size, .. } => size, _ => panic!() };
        assert_eq!((size(8), size(16), size(100)), (8, 16, 16));
    }
}
