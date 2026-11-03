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

fn material_sidebar(game: &Game<'_>, context: &MaterialBookContext, id: &str, section: &str) -> Vec<serde_json::Value> {
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
            let mut buckets = Vec::new(); let mut first_fluid = None;
            for (_, bytes) in &ordered {
                let Ok(raw) = serde_json::from_slice::<serde_json::Value>(bytes) else { continue; };
                if raw.get("hide_in_book").and_then(|v| v.as_bool()) == Some(true) { continue; }
                let Some(recipe) = crate::game::tconstruct_source_melting_recipes::MaterialFluidRecipe::parse(bytes, &conditions) else { continue; };
                if recipe.material != id { continue; }
                for fluid in fluid_ids(&recipe.fluid) {
                    let Some(bucket) = crate::game::tconstruct_source_fluid_container::filled_bucket(&fluid) else { continue; };
                    if !game.res.rest.get().is_some_and(|rest| rest.items.items.contains_key(bucket.as_str())) { continue; }
                    if first_fluid.is_none() { first_fluid = Some(fluid_name(&fluid)); }
                    buckets.push(bucket);
                }
            }
            buckets.sort(); buckets.dedup();
            if let Some(fluid) = first_fluid.filter(|_| !buckets.is_empty()) {
                let title = game.translate("book.tconstruct.material.castable", "Castable");
                let description = game.translate("book.tconstruct.material.cast_from", "Can be casted from %s in casts").replacen("%s", &fluid, 1);
                sidebar.push(serde_json::json!({"choices":buckets,"tooltip":[title,description]}));
            }
            let supported_types: std::collections::HashSet<_> = match section {
                "melee_harvest_materials" => ["tconstruct:head", "tconstruct:handle", "tconstruct:binding"].as_slice(),
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
                parts.sort();
                let Some(fluid) = fluid_ids(&recipe.fluid).first().map(|id| fluid_name(id)) else { continue; };
                if parts.is_empty() { continue; }
                let (ns, name) = recipe.input.split_once(':').unwrap_or(("tconstruct", &recipe.input));
                let material_name = game.translate(&format!("material.{ns}.{name}"), name);
                let description = game.translate("book.tconstruct.material.composite_from", "Can be casted from %s on %s parts").replacen("%s", &fluid, 1).replacen("%s", &material_name, 1);
                sidebar.push(serde_json::json!({"choices":parts,"material":recipe.input,"tooltip":[game.translate("book.tconstruct.material.composite", "Composite"), description]}));
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

fn dynamic_materials(game: &Game<'_>, key: &str) -> Option<Vec<(String, String)>> {
    let (definitions, _, tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref()?.as_ref().ok()?;
    let stats = game.world.chunks.__horizon_mixins_tconstruct_resolved_material_stats().as_ref()?.as_ref().ok()?;
    let kinds: &[&str] = match key {
        "melee_harvest_materials" => &["tconstruct:head", "tconstruct:handle", "tconstruct:binding"],
        "ranged_materials" => &["tconstruct:limb", "tconstruct:grip", "tconstruct:bowstring"],
        "ammo_materials" => &["tconstruct:arrow_head", "tconstruct:arrow_shaft", "tconstruct:fletching"],
        _ => return None,
    };
    let mut selected: Vec<_> = definitions.iter().filter(|(id, (_, hidden, tier, _, _))| {
        !hidden && (0..=1).contains(tier)
            && stats.get(*id).is_some_and(|s| kinds.iter().any(|kind| s.contains_key(*kind)))
            && if key == "ammo_materials" { tags.get("tconstruct:book/basic_ammo").is_some_and(|set| set.contains(*id)) }
               else { !tags.get("tconstruct:book/distant").is_some_and(|set| set.contains(*id)) }
    }).collect();
    selected.sort_by_key(|(id, (_, _, tier, order, _))| {
        let has = |kind: &str| stats.get(*id).is_some_and(|s| s.contains_key(kind));
        let group = match key {
            "melee_harvest_materials" => (i32::from(!has("tconstruct:head")), 0, 0),
            "ranged_materials" => (i32::from(!has("tconstruct:bowstring")), i32::from(has("tconstruct:limb")), 0),
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
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok());
    super::tconstruct_materials_book_recipe::resolve(&recipe, tags, grid_size)
}

pub(crate) fn source_pages(game: &Game<'_>, path: &std::path::Path, puny: bool) -> Option<Vec<Text>> {
    let root = if puny { "assets/tconstruct/book/puny_smelting/" } else { "assets/tconstruct/book/materials_and_you/" };
    let book_title = if puny { "Puny Smelting" } else { "Materials & You" };
    let index: serde_json::Value = serde_json::from_str(&read(path, &format!("{root}index.json"))?).ok()?;
    let language = read(path, &format!("{root}en_us/language.lang"))?;
    let labels: std::collections::HashMap<_,_> = language.lines().filter_map(|line| line.split_once('=')).collect();
    let colors = read(path, "assets/tconstruct/book/source-colors.json").and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok()).unwrap_or_default();
    let material_context = material_book_context(game);
    let mut chapters: Vec<(String, Vec<Text>)> = Vec::new();
    for chapter in index.as_array()? {
        let key = chapter.get("name").and_then(|x| x.as_str())?;
        // Only the source's introductory machine chapters are supported here;
        // the other book's material tiers and modifier layouts differ.
        if puny && !matches!(key, "intro" | "melter" | "moving_fluids" | "casting") { continue; }
        let heading = labels.get(key).copied().unwrap_or(key);
        let data = chapter.get("data").and_then(|x| x.as_str()).unwrap_or("no-load");
        if data == "no-load" {
            if key == "armor" {
                let item_tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok());
                if let Some(tools) = item_tags.and_then(|tags| tags.get("tconstruct:modifiable/book_armor/materials_and_you")).filter(|tools| !tools.is_empty()) {
                    let mut tools: Vec<_> = tools.iter().collect();
                    tools.sort();
                    let mut chapter_pages = Vec::new();
                    if let Some(subtext) = labels.get("armor.subtext") {
                        chapter_pages.extend(split_page(game, subtext, 17).into_iter().map(Text::literal));
                    }
                    for id in tools {
                        let (namespace, name) = id.split_once(':').unwrap_or(("tconstruct", id));
                        let title = game.translate(&format!("item.{namespace}.{name}"), name);
                        let source = format!("assets/tconstruct/book/encyclopedia/en_us/armor/{}.json", id.replace(':', "_"));
                        let value = read(path, &source).and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
                        chapter_pages.extend(tool_pages(game, &title, id, value.as_ref()));
                    }
                    if !chapter_pages.is_empty() { chapters.push((heading.to_owned(), chapter_pages)); }
                }
                continue;
            }
            if let Some(materials) = dynamic_materials(game, key) {
                let mut chapter_pages = Vec::new();
                let subtext = labels.get(format!("{key}.subtext").as_str()).copied().unwrap_or("");
                if !subtext.is_empty() { chapter_pages.extend(split_page(game, subtext, 17).into_iter().map(Text::literal)); }
                let visible: &[&str] = match key {
                    "melee_harvest_materials" => &["tconstruct:head", "tconstruct:handle", "tconstruct:binding"],
                    "ranged_materials" => &["tconstruct:limb", "tconstruct:grip", "tconstruct:bowstring"],
                    _ => &["tconstruct:arrow_head", "tconstruct:arrow_shaft", "tconstruct:fletching"],
                };
                for (id, name) in materials {
                    let mut groups: Vec<Option<serde_json::Value>> = vec![None; visible.len()];
                    let (namespace, material) = id.split_once(':').unwrap_or(("tconstruct", &id));
                    let base = material.split('#').next().unwrap_or(material);
                    let suffix = match key { "melee_harvest_materials" => "melee_harvest", "ranged_materials" => "ranged", _ => "ammo" };
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
                    let sidebar = material_sidebar(game, &material_context, &id, key);
                    let ranges: Vec<_> = groups.iter().map(|group| group.as_ref().and_then(|g| g.get("lines")).and_then(|v| v.as_array()).map(|lines| {
                        let continuation: Vec<_> = lines.iter().map(|line| line.get("continuation").and_then(|v| v.as_bool()).unwrap_or(false)).collect();
                        super::tconstruct_materials_book_layout::material_line_ranges(&continuation)
                    })).collect();
                    let count = ranges.iter().flatten().map(Vec::len).max().unwrap_or(1).max(1);
                    for chunk in 0..count {
                        let page_groups: Vec<_> = groups.iter().zip(&ranges).map(|(group, ranges)| group.as_ref().and_then(|group| {
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
                        if chunk + 1 == count && !flavor.is_empty() {
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
                if !chapter_pages.is_empty() { chapters.push((heading.to_owned(), chapter_pages)); }
            }
            continue;
        }
        let Some(section) = read(path, &format!("{root}{data}")) else { continue; };
        let Ok(section) = serde_json::from_str::<serde_json::Value>(&section) else { continue; };
        let mut chapter_pages = Vec::new();
        for page in section.as_array()? {
            let Some(relative) = page.get("data").and_then(|x| x.as_str()) else { continue; };
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
                        let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else { continue; };
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
            let mut graphic = value.get("image").and_then(|image| {
                let file = image.get("file")?.as_str()?;
                // Mantle permits omitted dimensions; the renderer falls back
                // to the atlas sprite's intrinsic width and height.
                Some(serde_json::json!({"image":file,"width":image.get("width").and_then(|v| v.as_u64()),"height":image.get("height").and_then(|v| v.as_u64())}))
            })
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
                    let id = super::tconstruct_materials_book_recipe::showcase_item(item)?;
                    if id == "tconstruct:creative_slot" {
                        let slot = item.pointer("/nbt/slot")?.as_str()?;
                        let source = match slot { "upgrades" => "upgrade", "abilities" => "ability", "defense" => "defense", "slotless" => "slotless", _ => return None };
                        Some(serde_json::json!({"source_icon":source}))
                    } else { Some(serde_json::json!({"showcase_item":id})) }
                });
            let mut text = String::new();
            if let Some(s) = value.get("subText").and_then(|v| v.as_str()) { text.push_str(s); }
            for key in ["text", "description", "properties"] {
                if let Some(parts) = value.get(key).and_then(|x| x.as_array()) {
                    if !text.is_empty() { text.push('\n'); }
                    super::tconstruct_materials_book_layout::append_parts(&mut text, parts, key == "properties");
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
                if (graphic.get("image").is_some() || graphic.get("source_icon").is_some() || graphic.get("showcase_item").is_some() || graphic.get("slots").is_some()) && !text.is_empty() {
                    let height = if graphic.get("source_icon").is_some() || graphic.get("showcase_item").is_some() { 40.0 }
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
        }
        if !chapter_pages.is_empty() { chapters.push((heading.to_owned(), chapter_pages)); }
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
    let puny = &*stack.item == "tconstruct:puny_smelting";
    if !puny && &*stack.item != "tconstruct:materials_and_you" { return false; }
    if remote || game.lan_acting.is_none() {
        let path = game.options.resources.clone();
        let Some(pages) = source_pages(game, &path, puny) else { return false; };
        *game.world.chunks.__horizon_mixins_tconstruct_materials_book_history_mut() = None;
        game.open_book(BookScreen::View { pages, page: usize::MAX, lectern: None });
        if remote {
            let hand = u8::from(game.hand == Hand::Off);
            game.send(S::CustomPayload(horizon_protocol::packets::configuration::CustomPayload {
                channel: "tconstruct:materials_book".into(), data: vec![hand],
            }));
        }
    }
    true
}
