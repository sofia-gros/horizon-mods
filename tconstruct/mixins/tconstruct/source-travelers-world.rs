// World-local recipe projection. No global material cache and no second
// inventory mutation path: actual native Recipes owns the complete result.
use super::tconstruct_source_travelers_crafting::{self as source, MaterialItem};
use horizon_client::{
    crafting::{Ingredient, Recipe, Template},
    items::ItemStack,
};
use std::collections::BTreeSet;
const PREFIX: &str = "tconstruct:travelers_native/";

pub(super) fn contribute(game: &super::Game<'_>, files: &mut Vec<(String, Vec<u8>)>) {
    let manual = game.options.resources.join("tconstruct-original-pack");
    let root = if manual.is_dir() {
        manual
    } else {
        game.options
            .resources
            .join("assets/tconstruct/source-packs/tconstruct-original-pack")
    };
    let pack = horizon_client::datapack::DataPack::builtin(&root);
    for name in ["goggles", "chestplate", "pants", "boots", "shield"] {
        let id = format!("tconstruct:tools/armor/travelers/{name}");
        let path = format!("recipe/{id}");
        if files.iter().any(|(key, _)| key == &path) {
            continue;
        }
        if let Some(bytes) = pack.__horizon_mixins_tconstruct_recipe_read(&format!(
            "data/tconstruct/recipes/tools/armor/travelers/{name}.json"
        )) {
            files.push((path, bytes));
        }
    }
}
fn ingredient_items(
    value: &serde_json::Value,
    tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    out: &mut BTreeSet<String>,
) {
    if let Some(values) = value.as_array() {
        for value in values {
            ingredient_items(value, tags, out);
        }
        return;
    }
    let Some(obj) = value.as_object() else {
        return;
    };
    // Instrument and component-sensitive recipes are not projected to item IDs.
    if obj.len() != 1 {
        return;
    }
    if let Some(id) = obj.get("item").and_then(serde_json::Value::as_str) {
        out.insert(id.into());
    }
    if let Some(tag) = obj
        .get("tag")
        .and_then(serde_json::Value::as_str)
        .and_then(|tag| tags.get(tag))
    {
        out.extend(tag.iter().cloned());
    }
}
fn generated(game: &super::Game<'_>) -> Option<Vec<(String, Recipe)>> {
    let chunks = &game.world.chunks;
    let recipes = chunks
        .__horizon_mixins_tconstruct_recipe_snapshot()
        .as_ref()?;
    let tags = chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    let known = chunks
        .__horizon_mixins_tconstruct_melter_known_items()
        .as_ref()?;
    let resources = chunks
        .__horizon_mixins_tconstruct_material_snapshot()
        .as_ref()?;
    let mut ids = BTreeSet::new();
    for bytes in recipes.values() {
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
            continue;
        };
        if value["type"].as_str() == Some("tconstruct:material") {
            ingredient_items(&value["ingredient"], tags, &mut ids);
        }
    }
    let mut candidates = Vec::new();
    for item in ids {
        if !known.contains(&item)
            || item == "minecraft:goat_horn"
            || tags
                .get("tconstruct:parts")
                .is_some_and(|values| values.contains(&item))
        {
            continue;
        }
        let Ok(Some(found)) = super::tconstruct_source_builder_world::lookup_material(
            &game.world,
            &ItemStack::new(&item, 1),
        ) else {
            continue;
        };
        candidates.push(MaterialItem {
            item,
            material: found.material,
            value: found.value,
            needed: found.needed,
        });
    }
    let tables = game.world_tables.as_ref()?;
    let mut generated = Vec::new();
    for (recipe_id, bytes) in recipes {
        // The legacy source snapshot uses recipes/, whereas this host's packs
        // use recipe/. A higher world layer in native tables must also be
        // authoritative, including replacement by an ordinary recipe.
        let path = format!("recipe/{recipe_id}");
        let override_bytes = tables
            .files
            .iter()
            .find(|(id, _)| id == &path)
            .filter(|(_, layers)| layers.len() > 1)
            .and_then(|(_, layers)| layers.last());
        let overridden;
        let bytes = if let Some(bytes) = override_bytes {
            // Unknown/conditional native-format overrides disable this bounded
            // projection rather than falling back to a lower source recipe.
            let Some(value) = super::tconstruct_source_melter_world::unconditional_recipe(bytes)
            else {
                continue;
            };
            overridden = value;
            &overridden
        } else {
            bytes
        };
        let Some(plans) =
            source::compile(bytes, &candidates, tags, known, |predicate, material| {
                super::tconstruct_source_melter_world::crafting_material_allowed(
                    &game.world,
                    predicate,
                    material,
                )
            })
        else {
            continue;
        };
        for plan in plans {
            let (namespace, name) = plan.result.split_once(':')?;
            let Some(definition) = resources
                .get(&format!("{namespace}:tinkering/tool_definitions/{name}"))
                .and_then(|layers| layers.last())
            else {
                continue;
            };
            let Some(output) = super::tconstruct_source_station_world::assemble(
                &game.world,
                definition,
                &plan.result,
                &plan.materials,
                None,
                &mut None,
            ) else {
                continue;
            };
            let hex = |value: &str| {
                value
                    .bytes()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            };
            let id = format!(
                "{PREFIX}{}/{}/{}",
                hex(recipe_id),
                hex(&plan.materials[0]),
                hex(&plan.materials[1])
            );
            if generated.len() >= 8192 {
                return None;
            }
            generated.push((
                id,
                Recipe::Shaped {
                    width: plan.width,
                    height: plan.height,
                    cells: plan
                        .cells
                        .into_iter()
                        .map(|items| items.map(Ingredient::of))
                        .collect(),
                    result: Template {
                        item: Some(output.item),
                        count: output.count,
                        damage: output.damage,
                        extra: output.extra,
                    },
                },
            ));
        }
    }
    Some(generated)
}
pub(super) fn refresh(game: &mut super::Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    let generated = generated(game).unwrap_or_default();
    let Some(tables) = game.world_tables.as_mut() else {
        return;
    };
    tables
        .recipes
        .crafting
        .retain(|(id, _)| !id.starts_with(PREFIX));
    // Keep ordinary source/native recipes first. Synthetic material variants
    // only supply the custom recipe kind absent from native normal matching.
    tables.recipes.crafting.extend(generated);
}
