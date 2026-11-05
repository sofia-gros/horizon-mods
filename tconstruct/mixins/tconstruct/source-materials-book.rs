// Source pages in Horizon's book screen, with separately drawn source images,
// recipe references, and material information from the resolved registry.
use super::*;

fn read(root: &std::path::Path, path: &str) -> Option<String> {
    std::fs::read_to_string(root.join(path)).ok()
}

// Match the available source page width and reserve vertical space for its title.
fn split_page(game: &Game<'_>, text: &str, rows: usize) -> Vec<String> {
    split_page_width(game, text, 182.0, rows)
}

fn split_page_width(game: &Game<'_>, text: &str, width: f32, rows: usize) -> Vec<String> {
    split_styled(game, &Text::literal(text), width, rows).into_iter().map(|page| {
        page.spans.iter().map(|span| span.text.as_str()).collect::<String>()
    }).collect()
}

fn split_styled(game: &Game<'_>, text: &Text, width: f32, rows: usize) -> Vec<Text> {
    let resolved = text.resolve_legacy();
    let lines = game.gui.atlas.font.split(&resolved, width);
    lines.chunks(rows.max(1)).map(|chunk| {
        let mut page = Text::empty();
        for (i, line) in chunk.iter().enumerate() {
            if i > 0 { page.push("\n"); }
            page.push(line.clone());
        }
        page
    }).collect()
}

fn item_choices(value: &serde_json::Value, tags: Option<&std::collections::HashMap<String, std::collections::HashSet<String>>>) -> Vec<String> {
        if let Some(values) = value.as_array() { return values.iter().flat_map(|v| item_choices(v, tags)).collect(); }
        let id = value.as_str().or_else(|| value.get("item").and_then(|v| v.as_str()));
        if let Some(id) = id.filter(|id| !id.starts_with('#')) { return vec![id.to_owned()]; }
        let Some(tag) = id.and_then(|id| id.strip_prefix('#')).or_else(|| value.get("tag").and_then(|v| v.as_str())) else { return Vec::new(); };
        let mut values: Vec<_> = tags.and_then(|tags| tags.get(tag)).into_iter().flatten().cloned().collect();
        values.sort(); values
    }

// The original guide prefers the building recipe with the most inputs and
// otherwise uses the definition's parts. Reuse the Station's source parser.
fn tool_inputs(game: &Game<'_>, id: &str) -> Vec<serde_json::Value> {
    let Some((namespace, name)) = id.split_once(':') else { return Vec::new(); };
    let Some(snapshot) = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref() else { return Vec::new(); };
    let Some(definition) = snapshot.get(&format!("{namespace}:tinkering/tool_definitions/{name}")).and_then(|layers| layers.last()) else { return Vec::new(); };
    let fallback = crate::game::tconstruct_source_station_recipes::BuildingRecipe {
        result: id.to_owned(), result_count: 1, layout: None, parts_override: None,
        extra_requirements: Vec::new(), extra_materials: Vec::new(),
    };
    let mut selected: Option<(Vec<String>, Vec<serde_json::Value>)> = None;
    if let Some(recipes) = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref() {
        let mut candidates: Vec<_> = recipes.iter().collect();
        candidates.sort_by(|a, b| a.0.cmp(b.0));
        for (_, bytes) in candidates {
            let Some(recipe) = crate::game::tconstruct_source_station_recipes::BuildingRecipe::parse(bytes) else { continue; };
            if recipe.result != id { continue; }
            let Some(parts) = recipe.parts(definition) else { continue; };
            if selected.as_ref().is_none_or(|(old, extra)| parts.len() + recipe.extra_requirements.len() > old.len() + extra.len()) {
                selected = Some((parts, recipe.extra_requirements));
            }
        }
    }
    let selected = selected.unwrap_or_else(|| (fallback.parts(definition).unwrap_or_default(), Vec::new()));
    let mut inputs: Vec<_> = selected.0.iter().enumerate().map(|(index, id)| serde_json::json!({"part":id,"index":index})).collect();
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok());
    for input in &selected.1 { inputs.push(serde_json::json!({"choices":item_choices(input, tags)})); }
    inputs
}

// Source material sidebars use the registered multipart tools and the same
// material-to-part support table as the Station transaction.
struct MaterialBookContext {
    supports: Vec<(String, std::collections::HashSet<String>)>,
    tools: Vec<(String, Vec<String>)>,
}

fn material_book_context(game: &Game<'_>) -> MaterialBookContext {
    let mut supports = Vec::new();
    if let (Some(Ok((definitions, _, _))), Some(Ok(stats))) = (
        game.world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref(),
        game.world.chunks.__horizon_mixins_tconstruct_resolved_material_stats().as_ref(),
    ) {
        let mut ordered: Vec<_> = definitions.iter().filter(|(_, (_, hidden, _, _, _))| !hidden).collect();
        ordered.sort_by(|a, b| (a.1.2, a.1.3, a.0).cmp(&(b.1.2, b.1.3, b.0)));
        for (id, _) in ordered {
            let Some(types) = stats.get(id) else { continue; };
            let keys = types.keys().cloned().collect();
            supports.push((id.clone(), crate::game::tconstruct_tx::source_supported_original_parts(&keys, false)));
        }
    }
    let mut by_result: std::collections::HashMap<String, Vec<crate::game::tconstruct_source_station_recipes::BuildingRecipe>> = Default::default();
    if let Some(recipes) = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref() {
        let mut ordered: Vec<_> = recipes.iter().collect(); ordered.sort_by(|a, b| a.0.cmp(b.0));
        for (_, bytes) in ordered {
            if let Some(recipe) = crate::game::tconstruct_source_station_recipes::BuildingRecipe::parse(bytes) {
                by_result.entry(recipe.result.clone()).or_default().push(recipe);
            }
        }
    }
    let mut tools = Vec::new();
    if let (Some(tags), Some(snapshot)) = (
        game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()),
        game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref(),
    ) {
        if let Some(ids) = tags.get("tconstruct:modifiable/multipart") {
            let mut ids: Vec<_> = ids.iter().collect(); ids.sort();
            for id in ids {
                if tags.get("tconstruct:modifiable/ancient").is_some_and(|ancient| ancient.contains(id)) { continue; }
                let Some((namespace, name)) = id.split_once(':') else { continue; };
                let Some(definition) = snapshot.get(&format!("{namespace}:tinkering/tool_definitions/{name}")).and_then(|layers| layers.last()) else { continue; };
                let mut selected: Option<(Vec<String>, usize)> = None;
                for recipe in by_result.get(id).into_iter().flatten() {
                    let Some(parts) = recipe.parts(definition) else { continue; };
                    let size = parts.len() + recipe.extra_requirements.len();
                    if selected.as_ref().is_none_or(|(_, old)| size > *old) { selected = Some((parts, size)); }
                }
                let parts = selected.map(|(parts, _)| parts).unwrap_or_else(|| {
                    let fallback = crate::game::tconstruct_source_station_recipes::BuildingRecipe { result:id.clone(), result_count:1, layout:None, parts_override:None, extra_requirements:Vec::new(), extra_materials:Vec::new() };
                    fallback.parts(definition).unwrap_or_default()
                });
                if !parts.is_empty() { tools.push((id.clone(), parts)); }
            }
        }
    }
    MaterialBookContext { supports, tools }
}

fn material_sidebar(game: &Game<'_>, context: &MaterialBookContext, id: &str, section: &str, encyclopedia: bool) -> Vec<serde_json::Value> {
    let mut sidebar = Vec::new();
    let definitions = game.world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref().and_then(|v| v.as_ref().ok());
    let item_tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok());
    let fluid_context = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref().and_then(|v| v.as_ref().ok());
    let config = game.world.chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref().and_then(|v| v.as_ref().ok());
    let registries: Option<std::collections::HashMap<_, _>> = item_tags.zip(fluid_context).map(|(items, (_, fluids))| [
        ("minecraft:item".to_owned(), items.clone()), ("minecraft:fluid".to_owned(), fluids.clone())
    ].into_iter().collect());
    let conditions = |values: &[serde_json::Value]| registries.as_ref().zip(config)
        .and_then(|(registries, config)| crate::game::tconstruct_tx::source_conditions_match(values, registries, config)).unwrap_or(false);
    if let Some((_, _, tags)) = definitions {
        let categories: &[(&str, &str, &str)] = match section {
            "melee_harvest_materials" => &[("melee_harvest/general", "hand_axe", "general"), ("melee_harvest/melee", "sword", "melee"), ("melee_harvest/harvest", "pickaxe", "harvest")],
            "ranged_materials" => &[("ranged/balanced", "fishing_rod", "balanced"), ("ranged/light", "crossbow", "light"), ("ranged/heavy", "longbow", "heavy")],
            _ => &[],
        };
        for (tag, tool, category) in categories {
            if tags.get(&format!("tconstruct:{tag}")).is_some_and(|members| members.contains(id)) {
                let key = format!("book.tconstruct.material.category.{category}");
                sidebar.push(serde_json::json!({"item":format!("tconstruct:{tool}"), "display_tool":true, "tooltip":[game.translate(&key, category), game.translate(&format!("{key}.description"), "")]}));
                break;
            }
        }
    }
    let mut repair = Vec::new();
    if let Some(recipes) = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref() {
        for bytes in recipes.values() {
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else { continue; };
            if value.get("type").and_then(|v| v.as_str()) != Some("tconstruct:material") || value.get("material").and_then(|v| v.as_str()) != Some(id) { continue; }
            if let Some(raw) = value.get("conditions") { if !raw.as_array().is_some_and(|values| conditions(values)) { continue; } }
            if let (Some(ingredient), Some(tags), Some(items)) = (value.get("ingredient"), item_tags, game.world.chunks.__horizon_mixins_tconstruct_melter_known_items().as_ref()) {
                if let Some(choices) = crate::game::tconstruct_tx::source_plain_ingredient_items(ingredient, items, tags) { repair.extend(choices); }
            }
        }
    }
    repair.sort(); repair.dedup();
    if let Some(rest) = game.res.rest.get() { repair.retain(|item| rest.items.items.contains_key(item.as_str())); } else { repair.clear(); }
    if !repair.is_empty() { sidebar.push(serde_json::json!({"choices":repair})); }
    if section == "ammo_materials" || definitions.and_then(|(defs, _, _)| defs.get(id)).is_some_and(|definition| definition.0) {
        sidebar.push(serde_json::json!({"item":"tconstruct:part_builder", "tooltip":[game.translate("book.tconstruct.material.craftable", "Part Builder"), game.translate("book.tconstruct.material.part_builder", "Can be made in the part builder")]}));
    }
    let Some((_, current)) = context.supports.iter().find(|(material, _)| material == id) else { return sidebar; };
    if section != "ammo_materials" {
        let fluids = fluid_context.map(|(_, tags)| tags);
        let fluid_ids = |spec: &crate::game::tconstruct_source_melting_recipes::FluidSpec| -> Vec<String> {
            let mut ids: Vec<_> = if spec.tagged { fluids.and_then(|tags| tags.get(&spec.id)).into_iter().flatten().cloned().collect() } else { vec![spec.id.clone()] };
            ids.retain(|id| !id.contains(":flowing_")); ids.sort(); ids
        };
        let fluid_name = |id: &str| -> String {
            let (ns, name) = id.split_once(':').unwrap_or(("minecraft", id));
            game.translate(&format!("fluid.{ns}.{name}"), name)
        };
        if let Some(recipes) = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref() {
            let mut ordered: Vec<_> = recipes.iter().collect(); ordered.sort_by(|a,b| a.0.cmp(b.0));
            let mut fluid_options = Vec::new();
            for (_, bytes) in &ordered {
                let Ok(raw) = serde_json::from_slice::<serde_json::Value>(bytes) else { continue; };
                if raw.get("hide_in_book").and_then(|v| v.as_bool()) == Some(true) { continue; }
                let Some(recipe) = crate::game::tconstruct_source_melting_recipes::MaterialFluidRecipe::parse(bytes, &conditions) else { continue; };
                if recipe.material != id { continue; }
                for fluid in fluid_ids(&recipe.fluid) {
                    let Some(bucket) = crate::game::tconstruct_source_fluid_container::filled_bucket(&fluid) else { continue; };
                    if !game.res.rest.get().is_some_and(|rest| rest.items.items.contains_key(bucket.as_str())) { continue; }
                    fluid_options.push((bucket, fluid_name(&fluid)));
                }
            }
            let title = game.translate("book.tconstruct.material.castable", "Castable");
            let description = game.translate("book.tconstruct.material.cast_from", "Can be casted from %s in casts");
            if let Some(entry) = super::tconstruct_materials_book_recipe::material_fluid_options(&fluid_options, &title, &description) {
                sidebar.push(entry);
            }
            let supported_types: std::collections::HashSet<_> = match section {
                "melee_harvest_materials" => ["tconstruct:head", "tconstruct:handle", "tconstruct:binding"].as_slice(),
                "armor_materials" if encyclopedia => super::tconstruct_materials_book_index::ENCYCLOPEDIA_ARMOR_STATS,
                "armor_materials" => ARMOR_MATERIAL_STATS,
                _ => ["tconstruct:limb", "tconstruct:grip", "tconstruct:bowstring"].as_slice(),
            }.iter().map(|v| (*v).to_owned()).collect();
            let page_parts = crate::game::tconstruct_tx::source_supported_original_parts(&supported_types, false);
            for (_, bytes) in &ordered {
                let Ok(raw) = serde_json::from_slice::<serde_json::Value>(bytes) else { continue; };
                if raw.get("hide_in_book").and_then(|v| v.as_bool()) == Some(true) { continue; }
                let Some(recipe) = crate::game::tconstruct_source_melting_recipes::CompositeFluidRecipe::parse(bytes, &conditions) else { continue; };
                if recipe.material != id || recipe.input == id { continue; }
                let Some((_, input)) = context.supports.iter().find(|(material, _)| material == &recipe.input) else { continue; };
                let mut parts: Vec<_> = current.intersection(input).filter(|part| page_parts.contains(*part)).cloned().collect();
                if let Ok((_, models)) = game.world.chunks.__horizon_mixins_tconstruct_render_assets().get_or_init(|| crate::game::tconstruct_source_render_info::load_render_assets(&game.options.resources)) { parts.retain(|part| models.contains_key(part)); } else { parts.clear(); }
                if let Some(rest) = game.res.rest.get() { parts.retain(|part| rest.items.items.contains_key(part.as_str())); } else { parts.clear(); }
                if parts.is_empty() { continue; }
                let fluids = fluid_ids(&recipe.fluid).iter().map(|id| fluid_name(id)).collect::<Vec<_>>();
                let (ns, name) = recipe.input.split_once(':').unwrap_or(("tconstruct", &recipe.input));
                let material_name = game.translate(&format!("material.{ns}.{name}"), name);
                let title = game.translate("book.tconstruct.material.composite", "Composite");
                let description = game.translate("book.tconstruct.material.composite_from", "Can be casted from %s on %s parts");
                if let Some(entry) = super::tconstruct_materials_book_recipe::material_composite_options(&parts, &fluids, &recipe.input, &material_name, &title, &description) {
                    sidebar.push(entry);
                }
            }
        }
    }
    for (tool, parts) in &context.tools {
        if sidebar.len() >= 9 { break; }
        let mut materials = Vec::new(); let mut used = false;
        for part in parts {
            if current.contains(part) { materials.push(id.to_owned()); used = true; }
            else if let Some((fallback, _)) = context.supports.iter().find(|(_, supported)| supported.contains(part)) { materials.push(fallback.clone()); }
            else { materials.clear(); break; }
        }
        if used && !materials.is_empty() { sidebar.push(serde_json::json!({"item":tool, "materials":materials})); }
    }
    sidebar
}

// ContentTool uses a description at the top, properties below on the left,
// and the source render tool on the right. Preserve any overflow on later pages.
fn tool_pages(game: &Game<'_>, title: &str, id: &str, data: Option<&serde_json::Value>) -> Vec<Text> {
    let mut description = String::new();
    let mut properties = String::new();
    if let Some(data) = data {
        if let Some(parts) = data.get("text").and_then(|v| v.as_array()) {
            super::tconstruct_materials_book_layout::append_parts(&mut description, parts, false);
        }
        if let Some(parts) = data.get("properties").and_then(|v| v.as_array()) {
            for part in parts {
                if let Some(value) = part.as_str().or_else(|| part.get("text").and_then(|v| v.as_str())) {
                    if !properties.is_empty() { properties.push('\n'); }
                    properties.push_str("• "); properties.push_str(value);
                }
            }
        }
    }
    let descriptions = split_styled(game, &Text::literal(&description), 172.0, 6);
    let property_pages = split_page_width(game, &properties, 86.0, 7);
    let spec = serde_json::json!({"item":id, "inputs":tool_inputs(game, id), "description_runs":descriptions.first().map(super::tconstruct_materials_book_layout::encode_runs), "properties":property_pages.first()});
    let mut result = vec![Text::styled(title, horizon_client::text::Style {
        insertion: Some(format!("tconstruct:book:{spec}")), underlined: Some(true), ..Default::default()
    })];
    result.extend(descriptions.into_iter().skip(1));
    result.extend(property_pages.into_iter().skip(1).map(Text::literal));
    result
}

// Armor materials can have more stat types than fit on one panel. Keep all
// supported armor types available for both selection and display.
const ARMOR_MATERIAL_STATS: &[&str] = &[
    "tconstruct:plating_helmet", "tconstruct:plating_chestplate",
    "tconstruct:plating_leggings", "tconstruct:plating_boots",
    "tconstruct:plating_shield", "tconstruct:maille", "tconstruct:shield_core",
    "tconstruct:cuirass", "tconstruct:slime", "tconstruct:skull",
    "tconstruct:shell", "tconstruct:laces", "tconstruct:ribcage",
];

// Keep the complete text recipe even when visual choices exceed the wire budget.
fn modifier_input_page(game: &Game<'_>, title: &str, inputs: &[Vec<String>]) -> Option<Text> {
    let mut spec = super::tconstruct_materials_book_recipe::modifier_input_spec(inputs)?;
    let preview_title = format!("{title}: Inputs");
    if game.gui.atlas.font.split(&Text::literal(&preview_title), 182.0).len() != 1 { return None; }
    let workstation = if inputs.len() > 3 { "Four or five inputs require a Tinker's Anvil." }
        else { "Repeated ingredients still need separate input slots." };
    let text = format!("One item per numbered input. Choices cycle every 3 seconds.\n\n{workstation}\n\nDetails and restrictions follow. This is not a crafting grid.");
    let body = split_styled(game, &Text::literal(text), 182.0, 11);
    if body.len() != 1 { return None; }
    spec["body_runs"] = serde_json::json!(super::tconstruct_materials_book_layout::encode_runs(&body[0]));
    let insertion = format!("tconstruct:book:{spec}");
    if insertion.len() > 8192 { return None; }
    Some(Text::styled(preview_title, horizon_client::text::Style { insertion:Some(insertion), underlined:Some(true), ..Default::default() }))
}

// A visible, deliberately partial modifier consumer. Read fresh world snapshots
// on each open/request; never turn a cached or missing registry into pages.
fn bonus_modifier_pages(game: &Game<'_>, tag: &str) -> Vec<Text> {
    let Some(raw) = game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref().and_then(|v| v.as_ref().ok()) else { return Vec::new(); };
    let Some(recipes) = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref() else { return Vec::new(); };
    let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()) else { return Vec::new(); };
    let Some(rest) = game.res.rest.get() else { return Vec::new(); };
    let registry = crate::game::tconstruct_source_upgrade_rules::bonus_guide_registry();
    let supported = registry.iter().map(|(id, _, _)| (*id).to_owned()).collect();
    let Ok(modifiers) = crate::game::tconstruct_modifier_snapshot::supported_guide_tag(raw, tag, &supported) else { return Vec::new(); };
    let items: Vec<String> = rest.items.items.keys().map(|id| id.to_string()).collect();
    let tools: Vec<String> = tags.get("tconstruct:modifiable").into_iter().flatten()
        .filter(|id| rest.items.items.contains_key(id.as_str())).cloned().collect();
    let entries = crate::game::tconstruct_modifier_snapshot::bonus_guide_recipes(recipes, &modifiers, &items, &tools,
        |ingredient, item| crate::game::tconstruct_source_upgrade_rules::ingredient_matches(ingredient, item, tags));
    let mut pages = Vec::new();
    for entry in entries {
        let Some((_, upgrades, abilities)) = registry.iter().find(|(id, _, _)| *id == entry.modifier) else { continue; };
        let (namespace, name) = entry.modifier.split_once(':').unwrap_or(("tconstruct", &entry.modifier));
        let title = game.translate(&format!("modifier.{namespace}.{name}"), name);
        if let Some(preview) = modifier_input_page(game, &title, &entry.inputs) { pages.push(preview); }
        let item_name = |id: &str| {
            let (namespace, name) = id.split_once(':').unwrap_or(("minecraft", id));
            game.translate(&format!("item.{namespace}.{name}"), &game.translate(&format!("block.{namespace}.{name}"), id))
        };
        let tools = entry.tools.iter().map(|id| item_name(id)).collect::<Vec<_>>().join(", ");
        let mut text = format!("Supported bonus modifier (partial guide).\n\nAdds {upgrades} upgrade slot(s) and {abilities} ability slot(s), once per tool.\n\nUse one item in EACH listed input slot. Repeated ingredients need separate slots.");
        for (index, choices) in entry.inputs.iter().enumerate() {
            text.push_str(&format!("\n\nInput {}: {}", index + 1, choices.iter().map(|id| item_name(id)).collect::<Vec<_>>().join(" OR ")));
        }
        if entry.returns_bottles {
            text.push_str(&format!("\n\nEach consumed {} returns one {}.", item_name("minecraft:dragon_breath"), item_name("minecraft:glass_bottle")));
        }
        if entry.inputs.len() > 3 { text.push_str("\n\nMore than three inputs require a Tinker's Anvil."); }
        if entry.crystal_alternative {
            text.push_str(&format!("\n\nAlternative input: one {} carrying {title} ({}), instead of ALL listed ingredients. This uses one input slot and adds the modifier once, under the same tool and level restrictions. Empty or different-modifier crystals do not work. No ingredient container is returned.", item_name("tconstruct:modifier_crystal"), entry.modifier));
        }
        text.push_str(&format!("\n\nEligible tool types: {tools}.\n\nRecipe: {}\n\nShows this port's supported bonus recipes, not all source modifiers. Existing modifier and tool state still affect application.", entry.recipe));
        for part in split_styled(game, &Text::literal(text), 182.0, 15) {
            let mut page = Text::styled(&title, horizon_client::text::Style { insertion: Some("tconstruct:book-title".into()), underlined: Some(true), ..Default::default() });
            page.push("\n\n"); page.push(part); pages.push(page);
        }
    }
    pages
}

fn ordinary_modifier_pages(game: &Game<'_>, source_tags: &[String]) -> Vec<Text> {
    let Some(raw) = game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref().and_then(|v| v.as_ref().ok()) else { return Vec::new(); };
    let Some(recipes) = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref() else { return Vec::new(); };
    let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()) else { return Vec::new(); };
    let Some(rest) = game.res.rest.get() else { return Vec::new(); };
    let supported = crate::game::tconstruct_source_upgrade_rules::ordinary_guide_registry().into_iter().collect();
    let mut modifiers = Vec::new();
    for tag in source_tags {
        if let Ok(ids) = crate::game::tconstruct_modifier_snapshot::supported_guide_tag(raw, tag, &supported) { modifiers.extend(ids); }
    }
    let items: Vec<String> = rest.items.items.keys().map(|id| id.to_string()).collect();
    let tools: Vec<String> = tags.get("tconstruct:modifiable").into_iter().flatten()
        .filter(|id| rest.items.items.contains_key(id.as_str())).cloned().collect();
    let entries = crate::game::tconstruct_modifier_snapshot::ordinary_guide_recipes_with_levels(recipes, &modifiers, &items, &tools,
        |ingredient, item| crate::game::tconstruct_source_upgrade_rules::stack_ingredient_matches(ingredient, &ItemStack::new(item, 1), tags),
        |ingredient, item| crate::game::tconstruct_source_upgrade_rules::ingredient_matches(ingredient, item, tags),
        |modifier, item| crate::game::tconstruct_source_upgrade_rules::tool_supported_with_tags(modifier, item, tags),
        crate::game::tconstruct_source_upgrade_rules::fluid_guide_policy,
        crate::game::tconstruct_source_upgrade_rules::ordinary_range_guide_policy);
    let mut pages = Vec::new();
    for entry in entries {
        let (namespace, name) = entry.modifier.split_once(':').unwrap_or(("tconstruct", &entry.modifier));
        let title = game.translate(&format!("modifier.{namespace}.{name}"), name);
        if let Some(preview) = modifier_input_page(game, &title, &entry.inputs) { pages.push(preview); }
        let item_name = |id: &str| {
            let (namespace, name) = id.split_once(':').unwrap_or(("minecraft", id));
            game.translate(&format!("item.{namespace}.{name}"), &game.translate(&format!("block.{namespace}.{name}"), id))
        };
        let mut text = String::from("Supported modifier recipe (partial guide).\n\n");
        let description = game.translate(&format!("modifier.{namespace}.{name}.description"), "");
        if !description.is_empty() && !description.contains('%') { text.push_str(&description); text.push_str("\n\n"); }
        if let Some((minimum, maximum)) = entry.level_range {
            text.push_str(&format!("Adds one level per application, for resulting modifier levels {minimum} to {maximum}. Each added level requires {} upgrade slot(s) and {} ability slot(s).\n\n", entry.upgrade_cost, entry.ability_cost));
            if minimum > 1 { text.push_str(&format!("Requires at least level {} already installed; this recipe cannot start from level zero.\n\n", minimum - 1)); }
            if let Some((_, upgrades, abilities)) = crate::game::tconstruct_source_upgrade_rules::bonus_guide_registry().iter().find(|(id, _, _)| *id == entry.modifier) {
                text.push_str(&format!("Each added level grants {upgrades} upgrade slot(s) and {abilities} ability slot(s).\n\n"));
            }
        } else if let Some(maximum) = entry.repeatable_maximum {
            text.push_str(&format!("Adds one level per application, up to this port's limit of {maximum} levels while slots remain. Each added level requires {} upgrade slot(s) and {} ability slot(s).\n\n", entry.upgrade_cost, entry.ability_cost));
        } else {
            text.push_str(&format!("Level 1. Requires {} upgrade slot(s) and {} ability slot(s).\n\n", entry.upgrade_cost, entry.ability_cost));
        }
        if entry.check_trait_level { text.push_str("Existing material-trait levels count toward this limit.\n\n"); }
        text.push_str("Use one item in EACH listed input slot. Repeated ingredients need separate slots.\n");
        for (index, choices) in entry.inputs.iter().enumerate() {
            let qualifier = if entry.empty_inputs[index] { "fresh empty container with no crafting remainder: " } else { "" };
            text.push_str(&format!("\nInput {}: {qualifier}{}", index + 1, choices.iter().map(|id| item_name(id)).collect::<Vec<_>>().join(" OR ")));
        }
        if entry.empty_inputs.iter().any(|empty| *empty) { text.push_str("\n\nContainer choices are checked as fresh empty items. Filled or otherwise modified containers may be rejected."); }
        if entry.inputs.len() > 3 { text.push_str("\n\nMore than three inputs require a Tinker's Anvil."); }
        if entry.crystal_alternative {
            text.push_str(&format!("\n\nAlternative input: one {} carrying {title} ({}), instead of ALL listed ingredients. This uses one input slot and adds one level under the same level, tool and slot requirements. Empty or different-modifier crystals do not work. No ingredient container is returned.", item_name("tconstruct:modifier_crystal"), entry.modifier));
        }
        text.push_str(&format!("\n\nEligible tool types: {}.\n\nRecipe: {}\n\nNot all source modifiers or recipe forms are shown. Existing modifier and tool state still affect application.",
            entry.tools.iter().map(|id| item_name(id)).collect::<Vec<_>>().join(", "), entry.recipe));
        for part in split_styled(game, &Text::literal(text), 182.0, 15) {
            let mut page = Text::styled(&title, horizon_client::text::Style { insertion:Some("tconstruct:book-title".into()), underlined:Some(true), ..Default::default() });
            page.push("\n\n"); page.push(part); pages.push(page);
        }
    }
    pages
}

// Source categories are reference information, not promises of implemented
// effects. Supported recipe pages retain the same current-world Station gates.
fn modifier_group_pages(game: &Game<'_>, path: &std::path::Path, root: &str, section: &serde_json::Value,
    labels: &std::collections::HashMap<&str, &str>, category: &str) -> Vec<Text> {
    if !matches!(category, "upgrades" | "abilities" | "defense") { return Vec::new(); }
    let upgrades = category == "upgrades";
    let Some(notice_title) = labels.get(format!("{category}.partial.notice_title").as_str()).filter(|value| !value.trim().is_empty()) else { return Vec::new(); };
    let Some(notice) = labels.get(format!("{category}.partial.notice").as_str()).filter(|value| !value.trim().is_empty()) else { return Vec::new(); };
    let Some(reference) = labels.get(format!("{category}.partial.reference").as_str()).filter(|value| !value.trim().is_empty()) else { return Vec::new(); };
    let Some(empty) = labels.get(format!("{category}.partial.empty").as_str()).filter(|value| !value.trim().is_empty()) else { return Vec::new(); };
    let Some(groups) = section.as_array() else { return Vec::new(); };
    let titled_pages = |title: &str, body: &str| -> Vec<Text> {
        split_styled(game, &Text::literal(body), 182.0, 15).into_iter().map(|part| {
            let mut page = Text::styled(title, horizon_client::text::Style { insertion:Some("tconstruct:book-title".into()), underlined:Some(true), ..Default::default() });
            page.push("\n\n"); page.push(part); page
        }).collect()
    };
    let mut pages = Vec::new();
    for group in groups {
        let Some(name) = group.get("name").and_then(|value| value.as_str()) else { continue; };
        let group_identity = match category {
            "upgrades" => super::tconstruct_materials_book_index::puny_upgrade_group(name),
            "abilities" => super::tconstruct_materials_book_index::mighty_ability_group(name),
            "defense" => super::tconstruct_materials_book_index::mighty_defense_group(name),
            _ => None,
        };
        let Some((tag, overview_path)) = group_identity else { continue; };
        // Do not place live recipes below an unrelated or replaced source group.
        if group.pointer("/extraData/tconstruct:load_modifiers/tag").and_then(|value| value.as_str()) != Some(tag) { continue; }
        let mut recipes = ordinary_modifier_pages(game, &[tag.to_owned()]);
        if upgrades { recipes.extend(incremental_modifier_pages(game, &[tag.to_owned()])); }
        let mut text = format!("{reference}\n\n");
        let title = if let Some(relative) = overview_path {
            if group.get("data").and_then(|value| value.as_str()) != Some(relative) { continue; }
            let Some(raw) = read(path, &format!("{root}en_us/{relative}")) else { continue; };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else { continue; };
            let Some(title) = value.get("title").and_then(|value| value.as_str()).filter(|title| !title.trim().is_empty()) else { continue; };
            let Some(paragraphs) = value.get("text").and_then(|value| value.as_array()) else { continue; };
            for paragraph in paragraphs {
                if let Some(body) = paragraph.get("text").and_then(|value| value.as_str()) {
                    if paragraph.get("paragraph").and_then(|value| value.as_bool()) == Some(true) { text.push_str("\n\n"); }
                    text.push_str(body);
                }
            }
            title.to_owned()
        } else {
            // Empty damage/armor subgroups have no source overview to preserve.
            if recipes.is_empty() { continue; }
            let Some(title) = labels.get(format!("{category}.{name}").as_str()).filter(|title| !title.trim().is_empty()) else { continue; };
            (*title).to_owned()
        };
        if recipes.is_empty() { text.push_str("\n\n"); text.push_str(empty); }
        pages.extend(titled_pages(&title, &text));
        pages.extend(recipes);
    }
    if !pages.is_empty() { pages.splice(0..0, titled_pages(notice_title, notice)); }
    pages
}

fn modifier_progress_page(game: &Game<'_>, title: &str, inputs: &[String], per_item: i32, needed: i32, leftover: Option<&str>) -> Option<Text> {
    let mut spec = super::tconstruct_materials_book_recipe::modifier_progress_spec(inputs, per_item, needed, leftover)?;
    let preview_title = format!("{title}: Progress");
    if game.gui.atlas.font.split(&Text::literal(&preview_title), 182.0).len() != 1 { return None; }
    let view = super::tconstruct_materials_book_recipe::modifier_progress_view(&spec, 0)?;
    let returns = if view.leftover.is_some() {
        format!("Return from zero: {} item(s).\nReturn icon identifies the item type.", view.returned_from_zero)
    } else { "No leftover item.".to_owned() };
    let text = format!("Each input: {} point(s).\nOne level: {} points.\nFrom zero: {} input item(s).\n{returns}\n\nPartial progress changes input and return counts. Operations stop at the next level. Full rules follow.", view.per_item, view.needed, view.from_zero);
    let body = split_styled(game, &Text::literal(text), 182.0, 11);
    if body.len() != 1 { return None; }
    spec["body_runs"] = serde_json::json!(super::tconstruct_materials_book_layout::encode_runs(&body[0]));
    let insertion = format!("tconstruct:book:{spec}");
    if insertion.len() > 8192 { return None; }
    Some(Text::styled(preview_title, horizon_client::text::Style { insertion:Some(insertion), underlined:Some(true), ..Default::default() }))
}

fn incremental_modifier_pages(game: &Game<'_>, source_tags: &[String]) -> Vec<Text> {
    let Some(raw) = game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref().and_then(|v| v.as_ref().ok()) else { return Vec::new(); };
    let Some(recipes) = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref() else { return Vec::new(); };
    let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()) else { return Vec::new(); };
    let Some(rest) = game.res.rest.get() else { return Vec::new(); };
    let registry = crate::game::tconstruct_source_upgrade_rules::incremental_guide_registry();
    let supported = registry.keys().cloned().collect();
    let mut modifiers = Vec::new();
    for tag in source_tags {
        if let Ok(ids) = crate::game::tconstruct_modifier_snapshot::supported_guide_tag(raw, tag, &supported) { modifiers.extend(ids); }
    }
    let items: Vec<String> = rest.items.items.keys().map(|id| id.to_string()).collect();
    let tools: Vec<String> = tags.get("tconstruct:modifiable").into_iter().flatten()
        .filter(|id| rest.items.items.contains_key(id.as_str())).cloned().collect();
    let entries = crate::game::tconstruct_modifier_snapshot::incremental_guide_recipes_with_slotless(recipes, &modifiers, &registry, &items, &tools,
        |ingredient, item| crate::game::tconstruct_source_upgrade_rules::ingredient_matches(ingredient, item, tags),
        |modifier, item| crate::game::tconstruct_source_upgrade_rules::tool_supported_with_tags(modifier, item, tags),
        crate::game::tconstruct_source_upgrade_rules::incremental_limits_supported,
        crate::game::tconstruct_source_upgrade_rules::slotless_incremental_limit);
    let mut pages = Vec::new();
    for entry in entries {
        let (namespace, name) = entry.modifier.split_once(':').unwrap_or(("tconstruct", &entry.modifier));
        let title = game.translate(&format!("modifier.{namespace}.{name}"), name);
        if let Some(preview) = modifier_progress_page(game, &title, &entry.inputs, entry.per_item, entry.needed, entry.leftover.as_deref()) { pages.push(preview); }
        let item_name = |id: &str| {
            let (namespace, name) = id.split_once(':').unwrap_or(("minecraft", id));
            game.translate(&format!("item.{namespace}.{name}"), &game.translate(&format!("block.{namespace}.{name}"), id))
        };
        let mut text = String::from("Supported incremental recipe (partial guide).\n\n");
        let description = game.translate(&format!("modifier.{namespace}.{name}.description"), "");
        if !description.is_empty() && !description.contains('%') { text.push_str(&description); text.push_str("\n\n"); }
        if entry.default_maximum { text.push_str(&format!("The source recipe has no explicit maximum; this port limits it to {} levels.\n\n", entry.maximum)); }
        if entry.upgrade_cost == 0 { text.push_str("Slotless: no upgrade or ability slots are consumed.\n\n"); }
        if entry.modifier == "tconstruct:blindshot" { text.push_str("Blindshot cannot be recovered through this port's modifier-extraction operation.\n\n"); }
        text.push_str(&format!("Each ingredient item adds {} progress point(s). {} points complete a level; maximum {} levels. Starting each level requires {} upgrade slot(s).\n\nPartial progress can be added over multiple Station operations. One ingredient operation stops at the next completed level.\n\nInput alternatives: {}.\n\nEligible tool types: {}.\n\nRecipe: {}\n\nOnly supported single-item and block-value recipes are shown. Existing modifier and tool state still affect application.",
            entry.per_item, entry.needed, entry.maximum, entry.upgrade_cost, entry.inputs.iter().map(|id| item_name(id)).collect::<Vec<_>>().join(" OR "),
            entry.tools.iter().map(|id| item_name(id)).collect::<Vec<_>>().join(", "), entry.recipe));
        let from_zero = entry.needed / entry.per_item + i32::from(entry.needed % entry.per_item != 0);
        text.push_str(&format!("\n\nFrom zero progress, {from_zero} input item(s) complete the next level."));
        if let Some(leftover) = &entry.leftover {
            text.push_str(&format!("\n\nWhen an item crosses the next level boundary, unused progress points return as {} (one item per unused point).", item_name(leftover)));
        }
        if entry.crystal_alternative {
            text.push_str(&format!("\n\nAlternative input: one {} carrying {title} ({}), used alone. Empty or different-modifier crystals do not work. One crystal adds {} progress points (one full level), preserving existing partial progress. It requires {} free upgrade slot(s), even when a partial level is installed, and the entire addition must fit within the level cap. This uses one input slot and returns no leftover. The same tool restrictions apply.", item_name("tconstruct:modifier_crystal"), entry.modifier, entry.needed, entry.upgrade_cost));
        }
        for part in split_styled(game, &Text::literal(text), 182.0, 15) {
            let mut page = Text::styled(&title, horizon_client::text::Style { insertion:Some("tconstruct:book-title".into()), underlined:Some(true), ..Default::default() });
            page.push("\n\n"); page.push(part); pages.push(page);
        }
    }
    pages
}

fn dynamic_materials(game: &Game<'_>, key: &str, puny: bool, mighty: bool, encyclopedia: bool) -> Option<Vec<(String, String)>> {
    let (definitions, _, tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref()?.as_ref().ok()?;
    let stats = game.world.chunks.__horizon_mixins_tconstruct_resolved_material_stats().as_ref()?.as_ref().ok()?;
    let whitestone_ingots = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
        .and_then(|v| v.as_ref().ok()).and_then(|tags| tags.get("tconstruct:whitestone_ingots"))
        .is_some_and(|items| !items.is_empty());
    let kinds: &[&str] = match key {
        "melee_harvest_materials" => &["tconstruct:head", "tconstruct:handle", "tconstruct:binding"],
        "ranged_materials" => &["tconstruct:limb", "tconstruct:grip", "tconstruct:bowstring"],
        "armor_materials" if encyclopedia => super::tconstruct_materials_book_index::ENCYCLOPEDIA_ARMOR_STATS,
        "armor_materials" => ARMOR_MATERIAL_STATS,
        "ammo_materials" => &["tconstruct:arrow_head", "tconstruct:arrow_shaft", "tconstruct:fletching"],
        _ => return None,
    };
    let mut selected: Vec<_> = definitions.iter().filter(|(id, (_, hidden, tier, _, _))| {
        let distant = tags.get("tconstruct:book/distant").is_some_and(|set| set.contains(*id));
        // Encyclopedia keeps the source default tier range, without the
        // progression books' distant/basic-ammo filters. Hidden entries and
        // unsupported stat groups still stay absent.
        let allowed = if encyclopedia {
            super::tconstruct_materials_book_index::encyclopedia_material_tier(*tier)
        } else if mighty {
            super::tconstruct_materials_book_index::mighty_material_tier(key, *tier, distant)
        } else if puny && key == "melee_harvest_materials" {
            super::tconstruct_materials_book_index::puny_melee_material_tier(*tier, distant, id, whitestone_ingots)
        } else {
            super::tconstruct_materials_book_index::material_book_tier(puny, key, *tier, distant,
                tags.get("tconstruct:book/basic_ammo").is_some_and(|set| set.contains(*id)))
        };
        !hidden && allowed
            && stats.get(*id).is_some_and(|s| kinds.iter().any(|kind| s.contains_key(*kind)))
    }).collect();
    selected.sort_by_key(|(id, (_, _, tier, order, _))| {
        let has = |kind: &str| stats.get(*id).is_some_and(|s| s.contains_key(kind));
        let group = match key {
            "melee_harvest_materials" => (i32::from(!has("tconstruct:head")), 0, 0),
            "ranged_materials" => (i32::from(!has("tconstruct:bowstring")), i32::from(has("tconstruct:limb")), 0),
            "armor_materials" if encyclopedia => (super::tconstruct_materials_book_index::encyclopedia_armor_order(
                has("tconstruct:cuirass"), has("tconstruct:maille"), has("tconstruct:plating_chestplate")), 0, 0),
            "armor_materials" => (i32::from(!has("tconstruct:plating_helmet")), 0, 0),
            _ => (i32::from(!has("tconstruct:arrow_head")), i32::from(has("tconstruct:fletching")), i32::from(has("tconstruct:arrow_shaft"))),
        };
        (group, *tier, *order, (*id).clone())
    });
    Some(selected.into_iter().map(|(id, _)| {
        let (namespace, name) = id.split_once(':').unwrap_or(("tconstruct", id));
        (id.clone(), game.translate(&format!("material.{namespace}.{name}"), name))
    }).collect())
}

fn recipe_graphic(game: &Game<'_>, id: &str, grid_size: &str) -> Option<serde_json::Value> {
    let recipes = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let recipe: serde_json::Value = serde_json::from_slice(recipes.get(id)?).ok()?;
    let rest = game.res.rest.get()?;
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok());
    let fluid_context = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref().and_then(|v| v.as_ref().ok());
    let config = game.world.chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref().and_then(|v| v.as_ref().ok());
    let registries: Option<std::collections::HashMap<_, _>> = tags.zip(fluid_context).map(|(items, (_, fluids))| [
        ("minecraft:item".to_owned(), items.clone()), ("minecraft:fluid".to_owned(), fluids.clone())
    ].into_iter().collect());
    let conditions = |values: &[serde_json::Value]| registries.as_ref().zip(config)
        .and_then(|(registries, config)| crate::game::tconstruct_tx::source_conditions_match(values, registries, config)).unwrap_or(false);
    super::tconstruct_materials_book_recipe::resolve(&recipe, tags, grid_size,
        |item| rest.items.items.contains_key(item), conditions)
}

pub(crate) fn source_pages(game: &Game<'_>, path: &std::path::Path, book: &str) -> Option<Vec<Text>> {
    let (root, book_title) = match book {
        "tconstruct:fantastic_foundry" => ("assets/tconstruct/book/fantastic_foundry/", "Fantastic Foundry"),
        "tconstruct:puny_smelting" => ("assets/tconstruct/book/puny_smelting/", "Puny Smelting"),
        "tconstruct:mighty_smelting" => ("assets/tconstruct/book/mighty_smelting/", "Mighty Smelting"),
        "tconstruct:tinkers_gadgetry" => ("assets/tconstruct/book/tinkers_gadgetry/", "Tinkers' Gadgetry"),
        "tconstruct:encyclopedia" => ("assets/tconstruct/book/encyclopedia/", "Encyclopedia"),
        "tconstruct:materials_and_you" => ("assets/tconstruct/book/materials_and_you/", "Materials & You"),
        _ => return None,
    };
    let foundry = book == "tconstruct:fantastic_foundry";
    let puny = book == "tconstruct:puny_smelting";
    let mighty = book == "tconstruct:mighty_smelting";
    let encyclopedia = book == "tconstruct:encyclopedia";
    let gadgetry = book == "tconstruct:tinkers_gadgetry";
    let index: serde_json::Value = serde_json::from_str(&read(path, &format!("{root}index.json"))?).ok()?;
    let language = read(path, &format!("{root}en_us/language.lang"))?;
    let labels: std::collections::HashMap<_,_> = language.lines().filter_map(|line| line.split_once('=')).collect();
    let colors = read(path, "assets/tconstruct/book/source-colors.json").and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok()).unwrap_or_default();
    let material_context = material_book_context(game);
    let mut chapters: Vec<(String, Vec<Text>)> = Vec::new();
    for chapter in index.as_array()? {
        let key = chapter.get("name").and_then(|x| x.as_str())?;
        // Gadgetry exposes only bounded, explicitly qualified exploration guidance.
        if gadgetry && !matches!(key, "intro" | "exploration") { continue; }
        let gadgetry_exploration = gadgetry && key == "exploration";
        if gadgetry_exploration && !super::tconstruct_materials_book_index::mighty_advice_ready([labels.get("exploration.partial").copied(), labels.get("exploration.partial.notice_title").copied(), labels.get("exploration.partial.notice").copied()]) { continue; }
        if gadgetry && !super::tconstruct_materials_book_index::mighty_advice_ready([labels.get("intro").copied(), labels.get("intro.partial.notice_title").copied(), labels.get("intro.partial.notice").copied()]) { continue; }
        // Reject unsupported transformer options instead of silently ignoring
        // pack-authored tier/tag filters, arrays, or an unrelated material type.
        let encyclopedia_material = if encyclopedia {
            chapter.pointer("/extraData/tconstruct:material_tier").and_then(|value| value.as_object())
                .filter(|settings| settings.len() == 2)
                .and_then(|settings| super::tconstruct_materials_book_index::encyclopedia_material_chapter(
                    key, chapter.get("data")?.as_str()?, settings.get("type")?.as_str()?, settings.get("detailed")?.as_bool()?))
        } else { None };
        if encyclopedia && key != "intro" && encyclopedia_material.is_none() { continue; }
        if encyclopedia && key == "intro" && !super::tconstruct_materials_book_index::mighty_advice_ready([labels.get("intro").copied(), labels.get("intro.partial.notice_title").copied(), labels.get("intro.partial.notice").copied()]) { continue; }
        let encyclopedia_heading = format!("{key}.partial");
        if encyclopedia_material.is_some() && !super::tconstruct_materials_book_index::mighty_advice_ready([
            labels.get(encyclopedia_heading.as_str()).copied(), labels.get("materials.partial.notice_title").copied(), labels.get("materials.partial.notice").copied()
        ]) { continue; }
        if foundry && !matches!(key, "intro" | "foundry") { continue; }
        // Mighty exposes selected source material chapters and partial broad-tool guidance.
        let mighty_advice = mighty && key == "tools";
        let mighty_smeltery = mighty && key == "smeltery";
        let mighty_entity = mighty && key == "entity_melting";
        let mighty_abilities = mighty && key == "abilities";
        let mighty_defense = mighty && key == "defense";
        let puny_upgrades = puny && key == "upgrades";
        let puny_slotless = puny && key == "slotless";
        if mighty && !matches!(key, "intro" | "tools" | "smeltery" | "melee_harvest_materials" | "ranged_materials" | "armor_materials" | "abilities" | "defense" | "entity_melting") { continue; }
        if mighty_advice && !super::tconstruct_materials_book_index::mighty_advice_ready([
            labels.get("tools.partial").copied(), labels.get("tools.partial.notice_title").copied(), labels.get("tools.partial.notice").copied()
        ]) { continue; }
        if mighty_smeltery && !super::tconstruct_materials_book_index::mighty_advice_ready([
            labels.get("smeltery.partial").copied(), labels.get("smeltery.partial.notice_title").copied(), labels.get("smeltery.partial.notice").copied()
        ]) { continue; }
        if mighty_entity && (!super::tconstruct_materials_book_index::mighty_advice_ready([
            labels.get("entity_melting.partial").copied(), labels.get("entity_melting.partial.notice_title").copied(), labels.get("entity_melting.partial.notice").copied()
        ]) || !labels.get("entity_melting.partial.reference").is_some_and(|value| !value.trim().is_empty())) { continue; }
        if mighty_abilities && !super::tconstruct_materials_book_index::mighty_advice_ready([
            labels.get("abilities.partial").copied(), labels.get("abilities.partial.notice_title").copied(), labels.get("abilities.partial.notice").copied()
        ]) { continue; }
        if mighty_defense && !super::tconstruct_materials_book_index::mighty_advice_ready([
            labels.get("defense.partial").copied(), labels.get("defense.partial.notice_title").copied(), labels.get("defense.partial.notice").copied()
        ]) { continue; }
        if puny_upgrades && !super::tconstruct_materials_book_index::mighty_advice_ready([
            labels.get("upgrades.partial").copied(), labels.get("upgrades.partial.notice_title").copied(), labels.get("upgrades.partial.notice").copied()
        ]) { continue; }
        if puny_slotless && (!super::tconstruct_materials_book_index::mighty_advice_ready([
            labels.get("slotless.partial").copied(), labels.get("slotless.partial.notice_title").copied(), labels.get("slotless.partial.notice").copied()
        ]) || ["slotless.partial.reference", "slotless.partial.empty"].iter()
            .any(|key| !labels.get(key).is_some_and(|value| !value.trim().is_empty()))) { continue; }
        // Puny and Mighty group bounded recipe projections under source overviews.
        // Other modifier forms and a complete registry still need more work.
        if puny && !matches!(key, "intro" | "melter" | "moving_fluids" | "casting" | "melee_harvest_materials" | "ranged_materials" | "armor_materials" | "armor" | "upgrades" | "slotless" | "worktable" | "further_reading") { continue; }
        let heading = if encyclopedia_material.is_some() { labels.get(encyclopedia_heading.as_str()).copied().unwrap_or(key) } else if mighty_entity { labels.get("entity_melting.partial").copied().unwrap_or("Entity Melting (Reference)") } else if mighty_defense { labels.get("defense.partial").copied().unwrap_or("Defense (Partial Guide)") } else if puny_slotless { labels.get("slotless.partial").copied().unwrap_or("Supported slotless (partial)") } else if puny_upgrades { labels.get("upgrades.partial").copied().unwrap_or("Supported upgrades (partial)") } else if mighty && key == "abilities" { labels.get("abilities.partial").copied().unwrap_or("Supported abilities (partial)") } else if gadgetry_exploration { labels.get("exploration.partial").copied().unwrap_or(key) } else if mighty_advice { labels.get("tools.partial").copied().unwrap_or(key) } else if mighty_smeltery { labels.get("smeltery.partial").copied().unwrap_or(key) } else { labels.get(key).copied().unwrap_or(key) };
        let data = chapter.get("data").and_then(|x| x.as_str()).unwrap_or("no-load");
        if data == "no-load" {
            if key == "armor" {
                let item_tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok());
                let armor_tag = if puny { "tconstruct:modifiable/book_armor/puny_smelting" } else { "tconstruct:modifiable/book_armor/materials_and_you" };
                if let Some(tools) = item_tags.and_then(|tags| tags.get(armor_tag)).filter(|tools| !tools.is_empty()) {
                    let mut tools: Vec<_> = tools.iter().collect();
                    tools.sort_by_key(|id| (if puny { super::tconstruct_materials_book_index::puny_armor_order(id) } else { usize::MAX }, (*id).clone()));
                    let mut chapter_pages = Vec::new();
                    if let Some(subtext) = labels.get("armor.subtext") {
                        chapter_pages.extend(split_page(game, subtext, 17).into_iter().map(Text::literal));
                    }
                    for id in tools {
                        let (namespace, name) = id.split_once(':').unwrap_or(("tconstruct", id));
                        let title = game.translate(&format!("item.{namespace}.{name}"), name);
                        let source = format!("assets/tconstruct/book/{}/en_us/armor/{}.json", if puny { "puny_smelting" } else { "encyclopedia" }, id.replace(':', "_"));
                        let value = read(path, &source).and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
                        chapter_pages.extend(tool_pages(game, &title, id, value.as_ref()));
                    }
                    if !chapter_pages.is_empty() { chapters.push((heading.to_owned(), chapter_pages)); }
                }
                continue;
            }
            let material_key = encyclopedia_material.unwrap_or(key);
            if let Some(materials) = dynamic_materials(game, material_key, puny, mighty, encyclopedia) {
                if !super::tconstruct_materials_book_index::show_material_chapter(puny || mighty || encyclopedia, materials.len()) { continue; }
                let mut chapter_pages = Vec::new();
                if encyclopedia {
                    let title = labels.get("materials.partial.notice_title").copied()?;
                    let note = labels.get("materials.partial.notice").copied()?;
                    chapter_pages.extend(split_styled(game, &Text::literal(note), 182.0, 15).into_iter().map(|part| {
                        let mut notice = Text::styled(title, horizon_client::text::Style { insertion:Some("tconstruct:book-title".into()), underlined:Some(true), ..Default::default() });
                        notice.push("\n\n"); notice.push(part); notice
                    }));
                }
                if mighty {
                    chapter_pages.extend(split_page(game, "Material entries reflect source data. Some materials, traits, or acquisition routes may be incomplete in this port.", 17).into_iter().map(Text::literal));
                }
                let subtext = labels.get(format!("{key}.subtext").as_str()).copied().unwrap_or("");
                if !subtext.is_empty() { chapter_pages.extend(split_page(game, subtext, 17).into_iter().map(Text::literal)); }
                let visible: &[&str] = match material_key {
                    "melee_harvest_materials" => &["tconstruct:head", "tconstruct:handle", "tconstruct:binding"],
                    "ranged_materials" => &["tconstruct:limb", "tconstruct:grip", "tconstruct:bowstring"],
                    "armor_materials" if encyclopedia => super::tconstruct_materials_book_index::ENCYCLOPEDIA_ARMOR_STATS,
                    "armor_materials" => ARMOR_MATERIAL_STATS,
                    _ => &["tconstruct:arrow_head", "tconstruct:arrow_shaft", "tconstruct:fletching"],
                };
                for (id, name) in materials {
                    let mut groups: Vec<Option<serde_json::Value>> = vec![None; visible.len()];
                    let (namespace, material) = id.split_once(':').unwrap_or(("tconstruct", &id));
                    let base = material.split('#').next().unwrap_or(material);
                    let suffix = match material_key { "melee_harvest_materials" => "melee_harvest", "ranged_materials" => "ranged", "armor_materials" => "armor", _ => "ammo" };
                    let specific = game.translate(&format!("material.{namespace}.{base}.flavor.{suffix}"), "");
                    let flavor = if specific.is_empty() { game.translate(&format!("material.{namespace}.{base}.flavor"), "") } else { specific };
                    let color_name = material.replace('#', ".");
                    let color_value = colors.get(format!("material.{namespace}")).and_then(|v| v.get(&color_name)).or_else(|| colors.get(format!("material.{namespace}")).and_then(|v| v.get(base)))
                        .and_then(|v| v.as_str()).and_then(|v| u32::from_str_radix(v.trim_start_matches('#'), 16).ok());
                    if let Some(snapshot) = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref() {
                        let stats_path = format!("{namespace}:tinkering/materials/stats/{base}");
                        let traits_path = format!("{namespace}:tinkering/materials/traits/{base}");
                        let raw_traits = snapshot.get(&traits_path).and_then(|layers| crate::game::tconstruct_tx::source_merge_material_traits(layers).ok());
                        if let Some(layers) = snapshot.get(&stats_path) {
                            if let Ok(stats) = crate::game::tconstruct_tx::source_merge_material_stats(layers) {
                                for (index, stat) in visible.iter().enumerate() {
                                    let Some(fields) = stats.get(*stat) else { continue; };
                                    let kind = stat.strip_prefix("tconstruct:").unwrap_or(stat);
                                    let part = game.translate(&format!("stat.tconstruct.{kind}"), kind);
                                    let mut lines = Vec::new();
                                    let mut keys: Vec<_> = crate::game::tconstruct_source_builder_display::field_order(stat).iter().copied().collect();
                                    for key in fields.keys() { if !keys.contains(&key.as_str()) { keys.push(key); } }
                                    for field in keys {
                                        let Some(value) = fields.get(field) else { continue; };
                                        let shown = value.as_f64().map(|number| super::tconstruct_materials_book_stat::number(kind, field, number)).or_else(|| value.as_str().map(|value| {
                                            if field == "mining_tier" { let key = format!("stat.tconstruct.harvest_tier.{}", value.replace(':', ".")); game.translate(&key, value) } else { value.to_owned() }
                                        }));
                                        if let Some(shown) = shown {
                                            let stat_name = match field { "melee_attack" | "melee_damage" => "attack_damage", "melee_speed" => "attack_speed", "mining_tier" => "harvest_tier", other => other };
                                            let label = game.translate(&format!("tool_stat.tconstruct.{stat_name}"), stat_name);
                                            let bonus = kind == "handle" || (kind == "limb" && matches!(field, "draw_speed" | "velocity" | "accuracy")) || (kind == "grip" && matches!(field, "durability" | "accuracy"));
                                            let mut text = Text::literal(label);
                                            if bonus { if let Some(number) = value.as_f64() { text.push(Text::colored(shown.clone(), super::tconstruct_materials_book_layout::bonus_color(number))); } else { text.push(shown.clone()); } }
                                            else { text.push(shown.clone()); }
                                            let description = game.translate(&format!("tool_stat.tconstruct.{stat_name}.description"), "");
                                            for (line_index, line) in game.gui.atlas.font.split(&text, 74.0).into_iter().enumerate() {
                                                lines.push(serde_json::json!({"runs":super::tconstruct_materials_book_layout::encode_runs(&line), "continuation":line_index > 0, "tooltip":if description.contains('%') { "" } else { &description }}));
                                            }
                                        }
                                    }
                                    if let Some(raw) = &raw_traits {
                                        let mut traits = std::collections::BTreeMap::new();
                                        crate::game::tconstruct_source_material_trait_select::add_traits(&mut traits, raw.default.as_ref(), &raw.per_stat, kind);
                                        for (trait_id, level) in traits {
                                            let (ns, trait_name) = trait_id.split_once(':').unwrap_or(("tconstruct", &trait_id));
                                            let title = game.translate(&format!("modifier.{ns}.{trait_name}"), trait_name);
                                            let text = Text::styled(format!("{title} {level}"), horizon_client::text::Style { color:Some(0x555555), underlined:Some(true), ..Default::default() });
                                            let description = game.translate(&format!("modifier.{ns}.{trait_name}.description"), "");
                                            for (line_index, line) in game.gui.atlas.font.split(&text, 74.0).into_iter().enumerate() {
                                                lines.push(serde_json::json!({"runs":super::tconstruct_materials_book_layout::encode_runs(&line), "continuation":line_index > 0, "tooltip":if description.contains('%') { "" } else { &description }}));
                                            }
                                        }
                                    }
                                    let stat_set = std::collections::HashSet::from([(*stat).to_owned()]);
                                    let mut parts: Vec<_> = crate::game::tconstruct_tx::source_supported_original_parts(&stat_set, false).into_iter().collect();
                                    if let Ok((_, models)) = game.world.chunks.__horizon_mixins_tconstruct_render_assets().get_or_init(|| crate::game::tconstruct_source_render_info::load_render_assets(&game.options.resources)) { parts.retain(|part| models.contains_key(part)); } else { parts.clear(); }
                                    parts.sort();
                                    groups[index] = Some(serde_json::json!({"title":part, "parts":parts, "lines":lines}));
                                }
                            }
                        }
                    }
                    let sidebar = material_sidebar(game, &material_context, &id, material_key, encyclopedia);
                    if material_key == "armor_materials" {
                        groups.retain(Option::is_some);
                        if groups.is_empty() { groups.push(None); }
                    }
                    let group_batches = groups.len().max(1).div_ceil(4);
                    for (batch_index, batch) in groups.chunks(4).enumerate() {
                        let ranges: Vec<_> = batch.iter().map(|group| group.as_ref().and_then(|g| g.get("lines")).and_then(|v| v.as_array()).map(|lines| {
                            let continuation: Vec<_> = lines.iter().map(|line| line.get("continuation").and_then(|v| v.as_bool()).unwrap_or(false)).collect();
                            super::tconstruct_materials_book_layout::material_line_ranges(&continuation)
                        })).collect();
                        let count = ranges.iter().flatten().map(Vec::len).max().unwrap_or(1).max(1);
                        for chunk in 0..count {
                            let page_groups: Vec<_> = batch.iter().zip(&ranges).map(|(group, ranges)| group.as_ref().and_then(|group| {
                                let mut group = group.clone();
                                let lines = group.get("lines").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                                let range = ranges.as_ref().and_then(|ranges| ranges.get(chunk)).cloned().unwrap_or(0..0);
                                if chunk > 0 && range.is_empty() { return None; }
                                group["lines"] = serde_json::json!(&lines[range]); Some(group)
                            })).collect();
                            let lengths: Vec<_> = page_groups.iter().map(|group| group.as_ref().and_then(|g| g.get("lines")).and_then(|v| v.as_array()).map(Vec::len)).collect();
                            let available = super::tconstruct_materials_book_layout::material_flavor_rows(&lengths);
                            let mut flavor_overflow = Vec::new();
                            let mut flavor_runs = serde_json::Value::Null;
                            if batch_index + 1 == group_batches && chunk + 1 == count && !flavor.is_empty() {
                                let text = Text::styled(format!("\"{flavor}\""), horizon_client::text::Style { italic:Some(true), ..Default::default() });
                                let pages = split_styled(game, &text, 155.0, available.max(1));
                                if available > 0 { if let Some(first) = pages.first() { flavor_runs = serde_json::json!(super::tconstruct_materials_book_layout::encode_runs(first)); } flavor_overflow.extend(pages.into_iter().skip(1)); }
                                else { flavor_overflow.extend(pages); }
                            }
                            let spec = serde_json::json!({"material_panel":{"material":id,"groups":page_groups,"sidebar":sidebar,"flavor_runs":flavor_runs}});
                            chapter_pages.push(Text::styled(&name, horizon_client::text::Style { insertion:Some(format!("tconstruct:book:{spec}")), color:color_value, underlined:Some(true), ..Default::default() }));
                            for part in flavor_overflow { chapter_pages.push(part); }
                        }
                    }
                }
                if !chapter_pages.is_empty() { chapters.push((heading.to_owned(), chapter_pages)); }
            }
            continue;
        }
        let Some(section) = read(path, &format!("{root}{data}")) else { continue; };
        let Ok(section) = serde_json::from_str::<serde_json::Value>(&section) else { continue; };
        if mighty_abilities || mighty_defense || puny_upgrades {
            let chapter_pages = modifier_group_pages(game, path, root, &section, &labels, key);
            if !chapter_pages.is_empty() { chapters.push((heading.to_owned(), chapter_pages)); }
            continue;
        }
        let mut chapter_pages = Vec::new();
        for page in section.as_array()? {
            let slotless_group = if puny_slotless {
                if let Some((tag, relative)) = page.get("name").and_then(|value| value.as_str())
                    .and_then(super::tconstruct_materials_book_index::puny_slotless_group) {
                    if page.pointer("/extraData/tconstruct:load_modifiers/tag").and_then(|value| value.as_str()) != Some(tag)
                        || page.get("data").and_then(|value| value.as_str()) != Some(relative) { continue; }
                    Some(tag)
                } else { None }
            } else { None };
            let group_recipes = match slotless_group {
                Some(tag) => {
                    let mut recipes = if tag == "tconstruct:slotless/bonus" { bonus_modifier_pages(game, tag) } else { Vec::new() };
                    recipes.extend(incremental_modifier_pages(game, &[tag.to_owned()]));
                    recipes
                }
                None => Vec::new(),
            };
            if gadgetry_exploration && !page.get("name").and_then(|v| v.as_str()).is_some_and(super::tconstruct_materials_book_index::gadgetry_exploration_page) { continue; }
            if foundry && key == "foundry" && !page.get("name").and_then(|v| v.as_str()).is_some_and(|name| matches!(name, "features" | "byproducts" | "structurereq" | "fgui1" | "fgui2" | "scorched_tanks")) { continue; }
            if mighty_advice && !page.get("name").and_then(|v| v.as_str()).is_some_and(super::tconstruct_materials_book_index::mighty_advice_page) { continue; }
            if mighty_smeltery && !page.get("name").and_then(|v| v.as_str()).is_some_and(super::tconstruct_materials_book_index::mighty_smeltery_page) { continue; }
            let Some(relative) = page.get("data").and_then(|x| x.as_str()) else { continue; };
            if mighty_entity && !super::tconstruct_materials_book_index::mighty_entity_melting_page(
                page.get("name").and_then(|value| value.as_str()).unwrap_or(""), relative,
                page.get("type").and_then(|value| value.as_str()).unwrap_or("")) { continue; }
            if relative == "no-load" {
                // Source tool sections supply navigation labels in language.lang;
                // keep them in the chapter index instead of silently discarding them.
                let group = if key == "tools" {
                    page.get("name").and_then(|v| v.as_str())
                        .and_then(|name| labels.get(format!("{key}.{name}").as_str()).copied())
                } else { None };
                let group_start = chapter_pages.len();
                let loads = page.get("extraData").and_then(|v| v.get("tconstruct:load_tools"));
                let list: Vec<_> = match loads { Some(serde_json::Value::Array(a)) => a.iter().collect(), Some(v) => vec![v], None => vec![] };
                for load in list {
                    let Some(tag) = load.get("tag").and_then(|v| v.as_str()) else { continue; };
                    let Some(tool_path) = load.get("path").and_then(|v| v.as_str()) else { continue; };
                    let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()) else { continue; };
                    let Some(tools) = tags.get(tag) else { continue; };
                    let mut tools: Vec<_> = tools.iter().collect(); tools.sort();
                    for id in tools {
                        let file = format!("{root}en_us/{tool_path}/{}.json", id.replace(':', "_"));
                        let Some(raw) = read(path, &file) else { continue; };
                        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&raw) else { continue; };
                        if mighty_advice {
                            let Some(note) = labels.get("tools.partial.tool_notice").filter(|note| !note.trim().is_empty()) else { continue; };
                            let Some(rest) = game.res.rest.get() else { continue; };
                            let registered = |item: &str| rest.items.items.contains_key(item);
                            let inputs = tool_inputs(game, id);
                            let inputs_available = !inputs.is_empty() && inputs.iter().all(|input| {
                                if let Some(part) = input.get("part").and_then(|v| v.as_str()) { return registered(part); }
                                input.get("choices").and_then(|v| v.as_array()).is_some_and(|choices|
                                    !choices.is_empty() && choices.iter().all(|choice| choice.as_str().is_some_and(&registered)))
                            });
                            if !super::tconstruct_materials_book_index::mighty_tool_guide_ready(id, registered(id), inputs_available,
                                value.get("tool").and_then(|v| v.as_str())) { continue; }
                            let Some(text) = value.get_mut("text").and_then(|v| v.as_array_mut()) else { continue; };
                            text.insert(0, serde_json::json!({"text":format!("{note}\n\n")}));
                        }
                        let fallback = id.split(':').next_back().unwrap_or(id);
                        let (namespace, name) = id.split_once(':').unwrap_or(("tconstruct", fallback));
                        let title = game.translate(&format!("item.{namespace}.{name}"), fallback);
                        chapter_pages.extend(tool_pages(game, &title, id, Some(&value)));
                    }
                }
                // The information group has ordinary pages following its marker.
                // Dynamic tool groups appear only when at least one tool loaded.
                if let Some(label) = group.filter(|_| chapter_pages.len() > group_start || page.get("name").and_then(|v| v.as_str()) == Some("group_info")) {
                    chapter_pages.insert(group_start, Text::literal(label));
                }
                continue;
            }
            let Some(raw) = read(path, &format!("{root}en_us/{relative}")) else { continue; };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else { continue; };
            let title = value.get("title").and_then(|x| x.as_str()).unwrap_or("");
            let reference_title = mighty_entity.then(|| format!("[Source] {title}"));
            let title = reference_title.as_deref().unwrap_or(title);
            let structure_diagram = if mighty_smeltery && page.get("name").and_then(|value| value.as_str()) == Some("structure") {
                if value.get("structure_diagram").and_then(|value| value.as_str()) != Some("small_smeltery") { continue; }
                let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_block_tags().as_ref().and_then(|value| value.as_ref().ok());
                let Some(rest) = game.res.rest.get() else { continue; };
                let Some(diagram) = super::tconstruct_materials_book_recipe::small_smeltery_spec(tags, |id| rest.items.items.contains_key(id)) else { continue; };
                Some(diagram)
            } else { None };
            let mut graphic = structure_diagram.or_else(|| value.get("image").and_then(|image| {
                let file = image.get("file")?.as_str()?;
                // Mantle permits omitted dimensions; the renderer falls back
                // to the atlas sprite's intrinsic width and height.
                Some(serde_json::json!({"image":file,"width":image.get("width").and_then(|v| v.as_u64()),"height":image.get("height").and_then(|v| v.as_u64())}))
            }))
                .or_else(|| value.get("recipe").and_then(|v| v.as_str()).and_then(|id| recipe_graphic(game, id, value.get("grid_size").and_then(|v| v.as_str()).unwrap_or("large"))))
                .or_else(|| {
                    let rows = value.get("grid")?.as_array()?;
                    let source_stack = |item: &serde_json::Value| -> Option<serde_json::Value> {
                        let id = item.get("item")?.as_str()?;
                        let nbt = item.get("nbt");
                        let damage = nbt.and_then(|v| v.get("Damage")).and_then(|v| v.as_u64()).unwrap_or(0);
                        let maximum = nbt.and_then(|v| v.get("tic_stats")).and_then(|v| v.get("tconstruct:durability")).and_then(|v| v.as_u64());
                        let count = item.get("amount_needed").and_then(|v| v.as_u64()).unwrap_or(1).clamp(1, u32::MAX as u64);
                        Some(serde_json::json!({"id":id, "damage":damage, "max_damage":maximum, "count":count}))
                    };
                    let source_choices = |value: &serde_json::Value| -> Option<serde_json::Value> {
                        if let Some(choices) = value.as_array() {
                            let choices: Vec<_> = choices.iter().filter_map(&source_stack).collect();
                            if choices.is_empty() { None } else { Some(serde_json::json!(choices)) }
                        } else { source_stack(value) }
                    };
                    let mut slots: Vec<Option<serde_json::Value>> = vec![None; 9];
                    for (y, row) in rows.iter().take(3).enumerate() {
                        for (x, item) in row.as_array()?.iter().take(3).enumerate() {
                            slots[y * 3 + x] = source_choices(item);
                        }
                    }
                    let result = value.get("result").and_then(source_choices);
                    let grid = if value.get("grid_size").and_then(|v| v.as_str()) == Some("small") { 2 } else { 3 };
                    Some(serde_json::json!({"slots":slots, "result":result, "grid":grid}))
                })
                .or_else(|| {
                    let item = value.get("item")?;
                    if item.get("tag").is_some() {
                        let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok());
                        let known = game.world.chunks.__horizon_mixins_tconstruct_melter_known_items().as_ref();
                        return super::tconstruct_materials_book_recipe::showcase_tag(item, tags, known);
                    }
                    let id = super::tconstruct_materials_book_recipe::showcase_item(item)?;
                    if id == "tconstruct:creative_slot" {
                        let slot = item.pointer("/nbt/slot")?.as_str()?;
                        let source = match slot { "upgrades" => "upgrade", "abilities" => "ability", "defense" => "defense", "slotless" => "slotless", _ => return None };
                        Some(serde_json::json!({"source_icon":source}))
                    } else { Some(serde_json::json!({"showcase_item":id})) }
                });
            // This chapter is a text-only source reference. Do not show an
            // unbundled image or let replacement resources imply live behavior.
            if mighty_entity { graphic = None; }
            let mut text = String::new();
            if let Some(s) = value.get("subText").and_then(|v| v.as_str()) { text.push_str(s); }
            for key in ["text", "description", "properties"] {
                if let Some(parts) = value.get(key).and_then(|x| x.as_array()) {
                    if !text.is_empty() { text.push('\n'); }
                    super::tconstruct_materials_book_layout::append_parts(&mut text, parts, key == "properties");
                }
            }
            if mighty_entity {
                text = format!("{}\n\n{text}", labels["entity_melting.partial.reference"]);
            }
            if puny_slotless {
                // Preserve the illustrated Damage Tools page and all its overflow,
                // while distinguishing every source page from supported recipes.
                text = format!("{}\n\n{text}", labels["slotless.partial.reference"]);
                if slotless_group.is_some() && group_recipes.is_empty() {
                    text.push_str("\n\n"); text.push_str(labels["slotless.partial.empty"]);
                }
            }
            // Carry rich text across wrapping and page breaks. Source inline
            // legacy formatting is resolved before measuring each line.
            let mut remaining = None;
            if let Some(mut graphic) = graphic.take() {
                if let Some(tooltip) = value.get("tooltip").and_then(|v| v.as_array()) {
                    let lines: Vec<_> = tooltip.iter().filter_map(|v| v.as_str().or_else(|| v.get("text").and_then(|s| s.as_str()))).take(16).collect();
                    if !lines.is_empty() { graphic["tooltip"] = serde_json::json!(lines); }
                }
                if (graphic.get("small_smeltery").is_some() || graphic.get("image").is_some() || graphic.get("source_icon").is_some() || graphic.get("showcase_item").is_some() || graphic.get("showcase_items").is_some() || graphic.get("slots").is_some()) && !text.is_empty() {
                    let height = if graphic.get("small_smeltery").is_some() { 80.0 }
                        else if graphic.get("source_icon").is_some() || graphic.get("showcase_item").is_some() || graphic.get("showcase_items").is_some() { 40.0 }
                        else if graphic.get("slots").is_some() { if graphic.get("grid").and_then(|v| v.as_u64()) == Some(2) { 78.0 } else { 114.0 } }
                        else { graphic.get("height").and_then(|v| v.as_u64()).unwrap_or(100).min(100) as f32 };
                    let rows = ((137.0 - height) / 9.0).floor().max(1.0) as usize;
                    let chunks = split_styled(game, &Text::literal(&text), 182.0, rows);
                    if let Some(first) = chunks.first() { graphic["body_runs"] = serde_json::json!(super::tconstruct_materials_book_layout::encode_runs(first)); }
                    remaining = Some(chunks.into_iter().skip(1).collect::<Vec<_>>());
                }
                chapter_pages.push(Text::styled(title, horizon_client::text::Style {
                    insertion: Some(format!("tconstruct:book:{}", graphic)), underlined: Some(true), ..Default::default()
                }));
            }
            if text.is_empty() { continue; }
            let parts = remaining.unwrap_or_else(|| split_styled(game, &Text::literal(&text), 182.0, if title.is_empty() { 17 } else { 15 }));
            for part in parts {
                if title.is_empty() { chapter_pages.push(part); }
                else {
                    let mut page = Text::styled(title, horizon_client::text::Style {
                        insertion: Some("tconstruct:book-title".into()), underlined: Some(true), ..Default::default()
                    });
                    page.push("\n\n");
                    page.push(part);
                    chapter_pages.push(page);
                }
            }
            chapter_pages.extend(group_recipes);
        }
        if !chapter_pages.is_empty() {
            if mighty_advice || mighty_smeltery || mighty_entity || encyclopedia || gadgetry || puny_slotless {
                let (title_key, note_key) = if mighty_entity { ("entity_melting.partial.notice_title", "entity_melting.partial.notice") } else if puny_slotless { ("slotless.partial.notice_title", "slotless.partial.notice") } else if gadgetry_exploration { ("exploration.partial.notice_title", "exploration.partial.notice") } else if encyclopedia || gadgetry { ("intro.partial.notice_title", "intro.partial.notice") } else if mighty_advice { ("tools.partial.notice_title", "tools.partial.notice") } else { ("smeltery.partial.notice_title", "smeltery.partial.notice") };
                if let (Some(title), Some(note)) = (labels.get(title_key), labels.get(note_key)) {
                    let notices = split_styled(game, &Text::literal(*note), 182.0, 15).into_iter().map(|part| {
                        let mut notice = Text::styled(*title, horizon_client::text::Style { insertion: Some("tconstruct:book-title".into()), underlined: Some(true), ..Default::default() });
                        notice.push("\n\n");
                        notice.push(part);
                        notice
                    });
                    chapter_pages.splice(0..0, notices);
                }
            }
            chapters.push((heading.to_owned(), chapter_pages));
        }
    }
    if chapters.is_empty() { return None; }
    // The host's existing book click handler understands ChangePage links.
    let sizes: Vec<_> = chapters.iter().map(|(_, pages)| pages.len()).collect();
    let (_, locations) = super::tconstruct_materials_book_index::layout(&sizes)?;
    let mut indexes = Vec::new();
    let mut pages = Vec::new();
    let mut contents = Text::styled(format!("{book_title}\n\n"), horizon_client::text::Style { insertion: Some("tconstruct:materials-book-root".into()), ..Default::default() });
    for (index, (heading, mut chapter)) in chapters.into_iter().enumerate() {
        if index > 0 && index % 5 == 0 {
            indexes.push(contents);
            contents = Text::literal(format!("{book_title}\n\n"));
        }
        let (chapter_page, first_page) = locations[index];
        let link = Text::styled(format!("{heading}\n"), horizon_client::text::Style {
            click: Some(horizon_client::text::ClickEvent::ChangePage(i32::try_from(chapter_page).ok()?)),
            underlined: Some(true), ..Default::default()
        });
        contents.push(link);
        let first_page = i32::try_from(first_page).ok()?;
        // Mantle's modifiers index links its source entries from the introductory
        // page. Resolve destinations after pagination so wrapped entries do not
        // shift later links, and use source titles rather than invented labels.
        if labels.get("modifiers").is_some_and(|label| *label == heading) {
            let source_index = read(path, &format!("{root}en_us/modifiers/index.json"))
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
            let index_title = source_index.as_ref().and_then(|v| v.get("title")).and_then(|v| v.as_str());
            let index_page = chapter.iter().position(|page| page.spans.first().is_some_and(|span| Some(span.text.as_str()) == index_title));
            let mut links = Vec::new();
            for name in ["traits", "upgrades", "slotless", "abilities", "defense"] {
                let source = read(path, &format!("{root}en_us/modifiers/{name}.json"))
                    .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
                let Some(title) = source.as_ref().and_then(|v| v.get("title")).and_then(|v| v.as_str()) else { continue; };
                let Some(offset) = chapter.iter().position(|page| page.spans.first().is_some_and(|span| span.text == title)) else { continue; };
                let destination = first_page.checked_add(i32::try_from(offset).ok()?)?;
                links.push((title.to_owned(), destination));
            }
            if let Some(page) = index_page.and_then(|index| chapter.get_mut(index)) {
                for (title, destination) in links {
                    page.push("\n");
                    page.push(Text::styled(title, horizon_client::text::Style {
                        click: Some(horizon_client::text::ClickEvent::ChangePage(destination)),
                        underlined: Some(true), ..Default::default()
                    }));
                }
            }
        }
        for (group, entries) in chapter.chunks(5).enumerate() {
            let mut section = Text::literal(format!("{heading}\n\n"));
            for (offset, entry) in entries.iter().enumerate() {
                let title = super::tconstruct_materials_book_index::label(entry.spans.first().map(|s| s.text.as_str()).unwrap_or(""));
                let destination = first_page + i32::try_from(group * 5 + offset).ok()?;
                section.push(Text::styled(format!("{title}\n"), horizon_client::text::Style {
                    click: Some(horizon_client::text::ClickEvent::ChangePage(destination)),
                    underlined: Some(true), ..Default::default()
                }));
            }
            pages.push(section);
        }
        pages.extend(chapter);
    }
    indexes.push(contents);
    indexes.extend(pages);
    Some(indexes)
}

pub(super) fn open(game: &mut Game<'_>, stack: &ItemStack, remote: bool) -> bool {
    let book = &*stack.item;
    if !matches!(book, "tconstruct:fantastic_foundry" | "tconstruct:puny_smelting" | "tconstruct:mighty_smelting" | "tconstruct:encyclopedia" | "tconstruct:tinkers_gadgetry" | "tconstruct:materials_and_you") { return false; }
    if remote || game.lan_acting.is_none() {
        let path = game.options.resources.clone();
        let Some(pages) = source_pages(game, &path, book) else { return false; };
        let sequence = game.world.chunks.__horizon_mixins_tconstruct_materials_book_request_sequence().wrapping_add(1);
        *game.world.chunks.__horizon_mixins_tconstruct_materials_book_request_sequence_mut() = sequence;
        *game.world.chunks.__horizon_mixins_tconstruct_materials_book_history_mut() = None;
        game.open_book(BookScreen::View { pages, page: usize::MAX, lectern: None });
        if remote {
            let hand = u8::from(game.hand == Hand::Off);
            let mut data = vec![hand];
            data.extend_from_slice(&sequence.to_be_bytes());
            game.send(S::CustomPayload(horizon_protocol::packets::configuration::CustomPayload {
                channel: "tconstruct:materials_book".into(), data,
            }));
        }
    }
    true
}
