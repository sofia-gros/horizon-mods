//! Refilling overslime in a crafting grid, the original's
//! `OverslimeCraftingTableRecipe` (SlimeKnights, MIT; see LICENSE), over
//! PlutoLib's special-recipe surface. The four shipped recipes are the
//! `tools/modifiers/slotless/overslime/*_bottle_crafting_table` files
//! (`tconstruct:crafting_overslime_modifier`): a durability tool plus
//! slime bottles in the grid gets its overslime refilled by the recipe's
//! restore amount per bottle (the module's add applies the overworked
//! bonus and the capacity cap, as in the original).
//!
//! The recipes are code-registered like their sibling
//! `tables/crafting_table_repair` (see crafting_repair.rs): the special
//! surface reports no recipe file, so each recipe's fields live here as
//! constants matching the original data files.
//!
//! Documented reduction: the original's `getRemainingItems` consumes
//! only as many bottles as the refill needs; the special-recipe surface
//! consumes one item from every occupied grid cell, so a grid holding
//! more bottles than the missing capacity still spends one per cell
//! (the refill itself is computed from every cell, as the original's
//! `assemble` does).

use std::sync::Arc;

use pluto_lib::recipe_api_v1::{
    GridFacts, SpecialRecipeRegistration, SpecialResult, StackView,
};
use pluto_lib::{Identifier, ItemSlot};

use crate::materials::Definitions as Materials;
use crate::modifiers::Registry;
use crate::tool_definitions::{self, Data};
use crate::{overslime, tool_slot, tool_stack};

/// `TinkerTags.Items.DURABILITY`, the recipes' `tools` requirement.
const DURABILITY_TAG: &str = "tconstruct:modifiable/durability";

/// One shipped recipe: its id, its bottle ingredient and its
/// `restore_amount`, transcribed from the original data file.
struct Recipe {
    id: &'static str,
    ingredient: &'static str,
    restore: i32,
}

const RECIPES: [Recipe; 4] = [
    Recipe {
        id: "tconstruct:tools/modifiers/slotless/overslime/earth_bottle_crafting_table",
        ingredient: "tconstruct:earth_slime_bottle",
        restore: 20,
    },
    Recipe {
        id: "tconstruct:tools/modifiers/slotless/overslime/ender_bottle_crafting_table",
        ingredient: "tconstruct:ender_slime_bottle",
        restore: 80,
    },
    Recipe {
        id: "tconstruct:tools/modifiers/slotless/overslime/ichor_bottle_crafting_table",
        ingredient: "tconstruct:ichor_slime_bottle",
        restore: 60,
    },
    Recipe {
        id: "tconstruct:tools/modifiers/slotless/overslime/sky_bottle_crafting_table",
        ingredient: "tconstruct:sky_slime_bottle",
        restore: 50,
    },
];

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

/// `OverslimeCraftingTableRecipe.matches` + `assemble` over the cropped
/// grid facts (the shared `findTool` scan): exactly one durability tool
/// and at least one bottle, nothing else; the tool either carries the
/// overslime modifier or still has capacity to refill.
fn grid_refill(loaded: &Loaded, recipe: &Recipe, grid: &GridFacts<'_>) -> Option<SpecialResult> {
    let ingredient_id = Identifier::parse(recipe.ingredient).ok()?;
    let durability = Identifier::parse(DURABILITY_TAG).ok()?;
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
    // No adding overslime via this recipe when the modifier is absent and
    // the shield is already at capacity: matches is level > 0 or
    // amount < capacity.
    let level = tool_stack::saved_modifiers(tool_data, &loaded.modifiers)
        .iter()
        .filter(|(entry, _)| entry.id == overslime::id())
        .fold(0, |sum, (entry, _)| sum.wrapping_add(entry.level));
    if level <= 0 && overslime::amount(tool_data) >= overslime::capacity(tool_data) {
        return None;
    }
    let tool_item = Identifier::parse(tool.id).ok()?;
    let definition = tool_definition(&tool_item)?;
    let env = loaded.environment(&tool_item, &definition);
    let mut result = ItemSlot::new(tool_item, tool.count);
    result.damage = tool.damage;
    result.custom_data = tool_data.clone();
    overslime::add_amount(&mut result.custom_data, &env, items_found * recipe.restore);
    tool_slot::sync_visuals(&mut result, &env).ok()?;
    Some(SpecialResult {
        item: tool.id.to_owned(),
        count: tool.count,
        damage: result.damage,
        custom_data: Some(result.custom_data),
    })
}

fn refill_callback<const N: usize>(grid: &GridFacts<'_>) -> Option<SpecialResult> {
    let loaded = load()?;
    grid_refill(&loaded, &RECIPES[N], grid)
}

/// Installs the four overslime crafting recipes as special recipes.
pub(crate) fn register() {
    let callbacks: [pluto_lib::recipe_api_v1::SpecialCallback; 4] = [
        refill_callback::<0>,
        refill_callback::<1>,
        refill_callback::<2>,
        refill_callback::<3>,
    ];
    for (recipe, callback) in RECIPES.iter().zip(callbacks) {
        let _ = pluto_lib::recipe_api_v1::register_special_recipe(SpecialRecipeRegistration {
            id: recipe.id,
            callback,
        });
    }
}
