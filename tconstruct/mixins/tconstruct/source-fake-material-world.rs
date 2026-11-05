// World-local projection of the two exact source fake-metal conversions.
use horizon_client::{crafting::{Ingredient, Recipe, Template}, items::ItemStack};
use horizon_world::nbt::Tag;
const PREFIX: &str = "tconstruct:fake_metal_native/";
const MARKER: &str = "tconstruct:fake_metal_conversion";

fn exact_source(bytes: &[u8], to_block: bool) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else { return false; };
    let expected = if to_block {
        serde_json::json!({"type":"tconstruct:crafting_shaped_materials", "category":"misc",
            "key":{"#":{"type":"tconstruct:material", "item":"tconstruct:fake_ingot",
                "material":{"type":"tconstruct:has_part", "part":"tconstruct:fake_storage_block"}}},
            "parts":"#", "pattern":["###","###","###"],
            "result":{"item":"tconstruct:fake_storage_block"}, "show_notification":true})
    } else {
        serde_json::json!({"type":"tconstruct:crafting_shapeless_materials", "category":"misc",
            "ingredients":[{"type":"tconstruct:material", "item":"tconstruct:fake_storage_block",
                "material":{"type":"tconstruct:has_part", "part":"tconstruct:fake_ingot"}}],
            "parts":1, "result":{"count":9,"item":"tconstruct:fake_ingot"}})
    };
    value == expected
}

fn generated(game: &super::Game<'_>) -> Option<Vec<(String, Recipe)>> {
    let chunks = &game.world.chunks;
    let recipes = chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let (definitions, _, _) = chunks.__horizon_mixins_tconstruct_resolved_material_definitions()
        .as_ref()?.as_ref().ok()?;
    let tables = game.world_tables.as_ref()?;
    if tables.items.get("tconstruct:fake_ingot").is_none()
        || tables.items.get("tconstruct:fake_storage_block").is_none() { return None; }
    let mut materials: Vec<_> = definitions.iter().filter(|(_, value)| !value.1)
        .map(|(id, _)| id).collect();
    materials.sort();
    if materials.len() > 512 { return None; }
    let mut generated = Vec::new();
    for (source_id, to_block) in [
        ("tconstruct:tools/parts/fake_ingot_to_block", true),
        ("tconstruct:tools/parts/fake_block_to_ingots", false),
    ] {
        let native_path = format!("recipe/{source_id}");
        // A native-format world replacement is authoritative even when it is
        // the only layer. Unsupported/conditional changes disable projection.
        let native = tables.files.iter().find(|(id, _)| id == &native_path)
            .and_then(|(_, layers)| layers.last());
        let Some(bytes) = native.or_else(|| recipes.get(source_id)) else { continue; };
        if !exact_source(bytes, to_block) { continue; }
        let (input, output, side, count) = if to_block {
            ("tconstruct:fake_ingot", "tconstruct:fake_storage_block", 3, 1)
        } else { ("tconstruct:fake_storage_block", "tconstruct:fake_ingot", 1, 9) };
        for material in &materials {
            if material.len() > 256 || material.contains('#')
                || super::tconstruct_source_render_info::material_id(material).as_deref() != Some(material.as_str()) {
                continue;
            }
            if !super::tconstruct_source_builder_world::fake_material_can_use(
                &game.world, &tables.items, material, output).ok()? { continue; }
            let mut stack = ItemStack::new(output, count);
            stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
                ("Material", Tag::String(material.as_str().into()))
            ])));
            let cells = (0..side * side).map(|_| {
                let mut ingredient = Ingredient::of(vec![input.into()]);
                ingredient.custom = Some((MARKER.into(), serde_json::Map::from_iter([
                    ("material".into(), serde_json::Value::String((*material).clone()))
                ])));
                Some(ingredient)
            }).collect();
            let material_hex: String = material.bytes().map(|b| format!("{b:02x}")).collect();
            let direction = if to_block { "block" } else { "ingots" };
            generated.push((format!("{PREFIX}{direction}/{material_hex}"), Recipe::Shaped {
                width: side, height: side, cells,
                result: Template { item: Some(stack.item), count: stack.count, damage: stack.damage, extra: stack.extra },
            }));
        }
    }
    Some(generated)
}

pub(super) fn refresh(game: &mut super::Game<'_>) {
    if game.remote.is_some() { return; }
    let generated = generated(game).unwrap_or_default();
    let Some(tables) = game.world_tables.as_mut() else { return; };
    // Incomplete or invalid snapshots remove stale synthetic recipes, but do
    // not alter other mods' tables or use a process-global eligibility cache.
    tables.recipes.crafting.retain(|(id, _)| !id.starts_with(PREFIX));
    tables.recipes.crafting.extend(generated);
    horizon_client::recipe_book::tconstruct_source_fake_material_book::replace(
        &mut tables.stations.recipe_index, &tables.recipes.crafting);
    restore_saved_book(game);
    game.tconstruct_invalidate_fake_material_book();
}

// A guest borrows the game's current player state; never restore the local
// player's saved knowledge into that borrowed book. update_world retries this
// one-shot handoff outside a guest scope if the first capture happened there.
pub(super) fn restore_saved_book(game: &mut super::Game<'_>) {
    if game.remote.is_some() || game.lan_acting.is_some()
        || game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().is_none() { return; }
    let Some(tables) = game.world_tables.as_ref() else { return; };
    let Some(pending) = game.world.chunks.__horizon_mixins_tconstruct_fake_material_book_pending_mut().take() else { return; };
    horizon_client::recipe_book::tconstruct_source_fake_material_book_load::restore(
        &mut game.level.recipe_book, &tables.stations.recipe_index, pending);
    game.tconstruct_invalidate_fake_material_book();
}

#[cfg(test)]
mod tests {
    use super::exact_source;
    #[test]
    fn only_complete_source_conversion_forms_are_projected() {
        let block = br#####"{"type":"tconstruct:crafting_shaped_materials","category":"misc","key":{"#":{"type":"tconstruct:material","item":"tconstruct:fake_ingot","material":{"type":"tconstruct:has_part","part":"tconstruct:fake_storage_block"}}},"parts":"#","pattern":["###","###","###"],"result":{"item":"tconstruct:fake_storage_block"},"show_notification":true}"#####;
        let ingots = br#####"{"type":"tconstruct:crafting_shapeless_materials","category":"misc","ingredients":[{"type":"tconstruct:material","item":"tconstruct:fake_storage_block","material":{"type":"tconstruct:has_part","part":"tconstruct:fake_ingot"}}],"parts":1,"result":{"count":9,"item":"tconstruct:fake_ingot"}}"#####;
        assert!(exact_source(block, true));
        assert!(exact_source(ingots, false));
        assert!(!exact_source(block, false));
        assert!(!exact_source(ingots, true));
        let mut value: serde_json::Value = serde_json::from_slice(ingots).unwrap();
        value["result"]["count"] = serde_json::json!(10);
        assert!(!exact_source(&serde_json::to_vec(&value).unwrap(), false));
        let mut value: serde_json::Value = serde_json::from_slice(block).unwrap();
        value["conditions"] = serde_json::json!([]);
        assert!(!exact_source(&serde_json::to_vec(&value).unwrap(), true));
        value.as_object_mut().unwrap().remove("conditions");
        value["key"]["#"]["material"]["part"] = serde_json::json!("tconstruct:tool_handle");
        assert!(!exact_source(&serde_json::to_vec(&value).unwrap(), true));
        assert!(!exact_source(b"invalid", true));
    }
}
