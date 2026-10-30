//! Station recipe selection using the current world's source data.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use super::tconstruct_source_station_recipes::{BuildingMatch, BuildingRecipe};

/// Until dynamic tool stats and wear are connected, only the existing fixed
/// pickaxe can be built. The source recipe must still match its actual inputs.
pub(super) fn building_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>]) -> Option<(BuildingMatch, ItemStack)> {
    let recipes = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let definition = materials.get("tconstruct:tinkering/tool_definitions/pickaxe")?.last()?;
    for bytes in recipes.values() {
        let Some(recipe) = BuildingRecipe::parse(bytes) else { continue; };
        if recipe.result != "tconstruct:pickaxe" { continue; }
        let Some(parts) = recipe.parts(definition) else { continue; };
        let Some(inputs) = slots.get(..3) else { return None; };
        let Some(matched) = recipe.match_inputs(&parts, parts.len(), inputs, |stack| {
            match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|tag| tag.get("Material")) {
                Some(Tag::String(material)) => Some(material.to_string_lossy().to_string()),
                _ => None,
            }
        }, |part, material| {
            super::tconstruct_source_builder_world::supported_material(world, material)
                .is_ok_and(|(_, supported)| supported.contains(part))
        }, |_, _| false) else { continue; };
        let mut preview = slots.to_vec();
        let mut output = None;
        if super::tconstruct_tx::take(&mut preview, &mut output, true, 24) {
            return Some((matched, output?));
        }
    }
    None
}
