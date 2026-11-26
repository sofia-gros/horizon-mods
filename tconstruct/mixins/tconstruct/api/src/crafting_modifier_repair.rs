//! Repair in a crafting grid for a tool carrying a named modifier, the
//! original's `ModifierRepairCraftingRecipe` (SlimeKnights, MIT; see
//! LICENSE), over PlutoLib's special-recipe surface. The one shipped
//! recipe is `tools/modifiers/tasty_crafting_table`
//! (`tconstruct:crafting_modifier_repair`): a durability tool with the
//! tasty modifier plus bacon in the grid comes back repaired by the
//! recipe's amount per bacon, scaled by the tasty level and passed
//! through the tool's REPAIR_FACTOR chain.
//!
//! The recipe is code-registered like its sibling
//! `tables/crafting_table_repair` (see crafting_repair.rs): the special
//! surface reports no recipe file, so the recipe's fields live here as
//! constants matching the original data file.
//!
//! Documented reduction: the original's `getRemainingItems` consumes
//! only as many ingredient cells as the repair needs; the special-recipe
//! surface consumes one item from every occupied grid cell, so a grid
//! holding more bacon than the damage needs still spends one per cell
//! (the repair itself is computed from every cell, as the original's
//! `assemble` does).

use std::sync::Arc;

use pluto_lib::recipe_api_v1::{
    GridFacts, SpecialRecipeRegistration, SpecialResult, StackView,
};
use pluto_lib::{Identifier, ItemSlot};

use crate::materials::Definitions as Materials;
use crate::modifiers::Registry;
use crate::tool_definitions::{self, Data};
use crate::{tool_slot, tool_stack};

/// The recipe id of the shipped `tools/modifiers/tasty_crafting_table`
/// recipe.
const RECIPE_ID: &str = "tconstruct:tools/modifiers/tasty_crafting_table";
/// The recipe's ingredient: `tconstruct:bacon`.
const INGREDIENT: &str = "tconstruct:bacon";
/// The modifier the tool must carry: `tconstruct:tasty`.
const MODIFIER: &str = "tconstruct:tasty";
/// The recipe's `repair_amount`.
const REPAIR_AMOUNT: i32 = 25;
/// `TinkerTags.Items.DURABILITY`.
const DURABILITY_TAG: &str = "tconstruct:modifiable/durability";

/// The server's loaded data; None while any registry has not loaded, so
/// a handler never answers from partial state.
struct Loaded {
    definitions: Materials,
    modifiers: Registry,
    items: Option<Arc<pluto_lib::RegistryLookup>>,
}

fn load() -> Option<Loaded> {
    let definitions = crate::common::register().read().ok()?.clone();
    if !definitions.server_loaded {
        return None;
    }
    let modifiers = crate::common::server_modifiers()?.read().ok()?.clone();
    Some(Loaded { definitions, modifiers, items: crate::common::item_snapshot() })
}

impl Loaded {
    fn in_tag(&self, item: &Identifier, tag: &Identifier) -> bool {
        self.items
            .as_ref()
            .and_then(|items| items.get_tag(tag))
            .is_some_and(|members| members.contains(item))
    }
    fn environment<'a>(
        &'a self,
        item: &'a Identifier,
        definition: &'a Data,
    ) -> tool_stack::Environment<'a> {
        tool_stack::Environment {
            definition: Some(definition),
            has_materials: definition.material_stat_types().is_some(),
            materials: &self.definitions,
            modifiers: &self.modifiers,
            item_tags_loaded: self.items.is_some(),
            item: Some(item),
            item_tags: self.items.as_deref(),
        }
    }
}

fn tool_definition(item: &Identifier) -> Option<Data> {
    tool_definitions::register().read().ok()?.data.get(item).cloned()
}

/// `IToolStackView.getModifierLevel` over the saved modifier list.
fn modifier_level(data: &pluto_lib::NbtCompound, modifiers: &Registry, id: &Identifier) -> i32 {
    tool_stack::saved_modifiers(data, modifiers)
        .iter()
        .filter(|(entry, _)| &entry.id == id)
        .fold(0, |sum, (entry, _)| sum.wrapping_add(entry.level))
}

/// `ModifierRepairCraftingRecipe.matches` + `assemble` over the cropped
/// grid facts (the shared `findTool` scan): exactly one durability tool
/// and at least one ingredient stack, nothing else; the tool is damaged
/// or broken and carries the modifier.
fn grid_repair(loaded: &Loaded, grid: &GridFacts<'_>) -> Option<SpecialResult> {
    let ingredient_id = Identifier::parse(INGREDIENT).ok()?;
    let durability = Identifier::parse(DURABILITY_TAG).ok()?;
    let modifier_id = Identifier::parse(MODIFIER).ok()?;
    let mut tool: Option<&StackView<'_>> = None;
    let mut items_found: i32 = 0;
    for cell in grid.cells.iter().flatten() {
        let item = Identifier::parse(cell.id).ok()?;
        if item == ingredient_id {
            items_found += 1;
        } else if loaded.in_tag(&item, &durability) {
            if tool.is_some() {
                return None;
            }
            tool = Some(cell);
        } else {
            return None;
        }
    }
    let tool = tool?;
    if items_found == 0 {
        return None;
    }
    let tool_data = tool.custom_data.and_then(|data| data.as_compound())?;
    if !tool_stack::is_broken(tool_data) && tool_stack::damage(tool_data) == 0 {
        return None;
    }
    let level = modifier_level(tool_data, &loaded.modifiers, &modifier_id);
    if level <= 0 {
        return None;
    }
    let tool_item = Identifier::parse(tool.id).ok()?;
    let definition = tool_definition(&tool_item)?;
    let env = loaded.environment(&tool_item, &definition);
    // assemble: items * repair_amount * level, through the REPAIR_FACTOR
    // chain; a non-positive result repairs nothing.
    let amount = items_found * REPAIR_AMOUNT * level;
    let amount = tool_stack::repair_factor(tool_data, &env, amount as f32);
    if amount <= 0.0 {
        return None;
    }
    let mut result = ItemSlot::new(tool_item.clone(), tool.count);
    result.damage = tool.damage;
    result.custom_data = tool_data.clone();
    tool_slot::repair_slot(&mut result, &env, amount as i32).ok()?;
    // getRemainingItems: the ingredient the tool's damage does not need stays in its
    // cell; one item repairs the chain's answer for a single item, or nothing when the
    // chain is non-positive (and then every ingredient cell is spent).
    let per_item = tool_stack::repair_factor(tool_data, &env, (REPAIR_AMOUNT * level) as f32);
    let per_item = if per_item > 0.0 { per_item as i32 } else { 0 };
    let remainders = crate::crafting_overslime::remaining_items(grid, &ingredient_id, tool_stack::damage(tool_data), per_item);
    Some(SpecialResult {
        item: tool.id.to_owned(),
        // The original's assemble returns the tool at count 1.
        count: 1,
        damage: result.damage,
        custom_data: Some(result.custom_data),
        remainders: Some(remainders),
    })
}

fn special_callback(grid: &GridFacts<'_>) -> Option<SpecialResult> {
    let loaded = load()?;
    grid_repair(&loaded, grid)
}

/// Installs the tasty crafting repair as a special recipe.
pub(crate) fn register() {
    let _ = pluto_lib::recipe_api_v1::register_special_recipe(SpecialRecipeRegistration {
        id: RECIPE_ID,
        callback: special_callback,
    });
}
