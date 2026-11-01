// Source pages in Horizon's book screen, with separately drawn source images,
// recipe references, and material information from the resolved registry.
use super::*;
use std::io::Read;

fn read(archive: &mut zip::ZipArchive<std::fs::File>, path: &str) -> Option<String> {
    let mut file = archive.by_name(path).ok()?;
    let mut value = String::new();
    file.read_to_string(&mut value).ok()?;
    Some(value)
}

fn split_page(text: &str, limit: usize) -> Vec<String> {
    let mut remaining = text;
    let mut result = Vec::new();
    while remaining.chars().count() > limit {
        let end = remaining.char_indices().nth(limit).map(|(i,_)| i).unwrap_or(remaining.len());
        let split = remaining[..end].rfind(char::is_whitespace).filter(|i| *i > 0).unwrap_or(end);
        result.push(remaining[..split].to_owned());
        remaining = remaining[split..].trim_start();
    }
    if !remaining.is_empty() { result.push(remaining.to_owned()); }
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

pub(crate) fn source_pages(game: &Game<'_>, path: &std::path::Path) -> Option<Vec<Text>> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).ok()?).ok()?;
    let root = "assets/tconstruct/book/materials_and_you/";
    let index: serde_json::Value = serde_json::from_str(&read(&mut zip, &format!("{root}index.json"))?).ok()?;
    let language = read(&mut zip, &format!("{root}en_us/language.lang"))?;
    let labels: std::collections::HashMap<_,_> = language.lines().filter_map(|line| line.split_once('=')).collect();
    let mut chapters: Vec<(String, Vec<Text>)> = Vec::new();
    for chapter in index.as_array()? {
        let key = chapter.get("name").and_then(|x| x.as_str())?;
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
                        chapter_pages.extend(split_page(subtext, 120).into_iter().map(Text::literal));
                    }
                    for id in tools {
                        let (namespace, name) = id.split_once(':').unwrap_or(("tconstruct", id));
                        let title = game.translate(&format!("item.{namespace}.{name}"), name);
                        chapter_pages.push(Text::styled(&title, horizon_client::text::Style {
                            insertion: Some(format!("tconstruct:book:{}", serde_json::json!({"item":id}))), ..Default::default()
                        }));
                        let source = format!("assets/tconstruct/book/encyclopedia/en_us/armor/{}.json", id.replace(':', "_"));
                        if let Some(raw) = read(&mut zip, &source) {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
                                let mut text = title.clone();
                                for field in ["text", "properties"] {
                                    if let Some(parts) = value.get(field).and_then(|v| v.as_array()) {
                                        for part in parts {
                                            if let Some(content) = part.as_str().or_else(|| part.get("text").and_then(|v| v.as_str())) {
                                                text.push('\n'); text.push_str(content);
                                            }
                                        }
                                    }
                                }
                                chapter_pages.extend(split_page(&text, 120).into_iter().map(Text::literal));
                            }
                        }
                    }
                    if !chapter_pages.is_empty() { chapters.push((heading.to_owned(), chapter_pages)); }
                }
                continue;
            }
            if let Some(materials) = dynamic_materials(game, key) {
                let mut chapter_pages = Vec::new();
                let subtext = labels.get(format!("{key}.subtext").as_str()).copied().unwrap_or("");
                if !subtext.is_empty() { chapter_pages.extend(split_page(subtext, 120).into_iter().map(Text::literal)); }
                for (id, name) in materials {
                    let mut detail = name.clone();
                    if let Some((namespace, material)) = id.split_once(':') {
                        let flavor = game.translate(&format!("material.{namespace}.{material}.flavor"), "");
                        if !flavor.is_empty() { detail.push_str("\n\n"); detail.push_str(&flavor); }
                        if let Some(snapshot) = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref() {
                            let stats_path = format!("{namespace}:tinkering/materials/stats/{material}");
                            let traits_path = format!("{namespace}:tinkering/materials/traits/{material}");
                            let raw_traits = snapshot.get(&traits_path).and_then(|layers| crate::game::tconstruct_tx::source_merge_material_traits(layers).ok());
                            if let Some(layers) = snapshot.get(&stats_path) {
                                if let Ok(stats) = crate::game::tconstruct_tx::source_merge_material_stats(layers) {
                                    let visible: &[&str] = match key {
                                        "melee_harvest_materials" => &["tconstruct:head", "tconstruct:handle", "tconstruct:binding"],
                                        "ranged_materials" => &["tconstruct:limb", "tconstruct:grip", "tconstruct:bowstring"],
                                        _ => &["tconstruct:arrow_head", "tconstruct:arrow_shaft", "tconstruct:fletching"],
                                    };
                                    for stat in visible {
                                        let Some(fields) = stats.get(*stat) else { continue; };
                                        let kind = stat.strip_prefix("tconstruct:").unwrap_or(stat);
                                        let heading = game.translate(&format!("stat.tconstruct.{kind}"), kind);
                                        detail.push_str(&format!("\n\n{heading}"));
                                        for (field, value) in fields {
                                            let shown = value.as_f64().map(|number| number.to_string()).or_else(|| value.as_str().map(str::to_owned));
                                            if let Some(shown) = shown {
                                                let label = game.translate(&format!("tool_stat.tconstruct.{field}"), field);
                                                detail.push_str(&format!("\n{label}{shown}"));
                                            }
                                        }
                                        if let Some(raw) = &raw_traits {
                                            let mut traits = std::collections::BTreeMap::new();
                                            crate::game::tconstruct_source_material_trait_select::add_traits(&mut traits, raw.default.as_ref(), &raw.per_stat, kind);
                                            for (trait_id, level) in traits {
                                                let (ns, id) = trait_id.split_once(':').unwrap_or(("tconstruct", &trait_id));
                                                let title = game.translate(&format!("modifier.{ns}.{id}"), id);
                                                detail.push_str(&format!("\n{title} {level}"));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    for page in split_page(&detail, 120) { chapter_pages.push(Text::literal(page)); }
                }
                if !chapter_pages.is_empty() { chapters.push((heading.to_owned(), chapter_pages)); }
            }
            continue;
        }
        let Some(section) = read(&mut zip, &format!("{root}{data}")) else { continue; };
        let Ok(section) = serde_json::from_str::<serde_json::Value>(&section) else { continue; };
        let mut chapter_pages = Vec::new();
        for page in section.as_array()? {
            let Some(relative) = page.get("data").and_then(|x| x.as_str()) else { continue; };
            if relative == "no-load" {
                let loads = page.get("extraData").and_then(|v| v.get("tconstruct:load_tools"));
                let list: Vec<_> = match loads { Some(serde_json::Value::Array(a)) => a.iter().collect(), Some(v) => vec![v], None => vec![] };
                for load in list {
                    let Some(tag) = load.get("tag").and_then(|v| v.as_str()) else { continue; };
                    let Some(path) = load.get("path").and_then(|v| v.as_str()) else { continue; };
                    let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()) else { continue; };
                    let Some(tools) = tags.get(tag) else { continue; };
                    let mut tools: Vec<_> = tools.iter().collect(); tools.sort();
                    for id in tools {
                        let file = format!("{root}en_us/{path}/{}.json", id.replace(':', "_"));
                        let Some(raw) = read(&mut zip, &file) else { continue; };
                        let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else { continue; };
                        let fallback = id.split(':').next_back().unwrap_or(id);
                        let (namespace, name) = id.split_once(':').unwrap_or(("tconstruct", fallback));
                        let title = game.translate(&format!("item.{namespace}.{name}"), fallback);
                        chapter_pages.push(Text::styled(&title, horizon_client::text::Style {
                            insertion: Some(format!("tconstruct:book:{}", serde_json::json!({"item":id}))), ..Default::default()
                        }));
                        let mut text = title;
                        for field in ["text", "properties"] {
                            if let Some(parts) = value.get(field).and_then(|v| v.as_array()) {
                                for part in parts {
                                    if let Some(s) = part.as_str().or_else(|| part.get("text").and_then(|v| v.as_str())) {
                                        text.push('\n'); text.push_str(s);
                                    }
                                }
                            }
                        }
                        chapter_pages.extend(split_page(&text, 120).into_iter().map(Text::literal));
                    }
                }
                continue;
            }
            let Some(raw) = read(&mut zip, &format!("{root}en_us/{relative}")) else { continue; };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else { continue; };
            let title = value.get("title").and_then(|x| x.as_str()).unwrap_or("");
            let graphic = value.get("image").and_then(|image| {
                let file = image.get("file")?.as_str()?;
                Some(serde_json::json!({"image":file,"width":image.get("width")?.as_u64()?,"height":image.get("height")?.as_u64()?}))
            })
                .or_else(|| value.get("recipe").and_then(|v| v.as_str()).and_then(|id| recipe_graphic(game, id, value.get("grid_size").and_then(|v| v.as_str()).unwrap_or("large"))))
                .or_else(|| {
                    let rows = value.get("grid")?.as_array()?;
                    let mut slots: Vec<Option<String>> = vec![None; 9];
                    for (y, row) in rows.iter().take(3).enumerate() {
                        for (x, item) in row.as_array()?.iter().take(3).enumerate() {
                            slots[y * 3 + x] = item.get("item").and_then(|v| v.as_str()).map(str::to_owned);
                        }
                    }
                    let result = value.get("result").and_then(|v| v.get("item")).and_then(|v| v.as_str());
                    let grid = if value.get("grid_size").and_then(|v| v.as_str()) == Some("small") { 2 } else { 3 };
                    Some(serde_json::json!({"slots":slots, "result":result, "grid":grid}))
                })
                .or_else(|| value.get("item").and_then(|v| v.get("item")).and_then(|v| v.as_str()).map(|id| serde_json::json!({"item":id})));
            if let Some(graphic) = graphic {
                chapter_pages.push(Text::styled(title, horizon_client::text::Style {
                    insertion: Some(format!("tconstruct:book:{}", graphic)), ..Default::default()
                }));
            }
            let mut text = String::new();
            if let Some(s) = value.get("subText").and_then(|v| v.as_str()) { text.push_str(s); }
            for key in ["text", "description", "properties", "tooltip"] {
                if let Some(parts) = value.get(key).and_then(|x| x.as_array()) {
                    for part in parts {
                        if let Some(s) = part.as_str().or_else(|| part.get("text").and_then(|x| x.as_str())) {
                            if !text.is_empty() { text.push('\n'); }
                            text.push_str(s);
                        }
                    }
                }
            }
            if text.is_empty() { continue; }
            for part in split_page(&text, 120) {
                chapter_pages.push(Text::literal(if title.is_empty() { part } else { format!("{title}\n\n{part}") }));
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
    let mut contents = Text::styled("Materials & You\n\n", horizon_client::text::Style { insertion: Some("tconstruct:materials-book-root".into()), ..Default::default() });
    for (index, (heading, chapter)) in chapters.into_iter().enumerate() {
        if index > 0 && index % 5 == 0 {
            indexes.push(contents);
            contents = Text::literal("Materials & You\n\n");
        }
        let (chapter_page, first_page) = locations[index];
        let link = Text::styled(format!("{heading}\n"), horizon_client::text::Style {
            click: Some(horizon_client::text::ClickEvent::ChangePage(i32::try_from(chapter_page).ok()?)),
            underlined: Some(true), ..Default::default()
        });
        contents.push(link);
        let first_page = i32::try_from(first_page).ok()?;
        for (group, entries) in chapter.chunks(5).enumerate() {
            let mut section = Text::literal(format!("{heading}\n\n"));
            for (offset, entry) in entries.iter().enumerate() {
                let title = entry.spans.first().and_then(|s| s.text.lines().next()).unwrap_or("");
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
    if &*stack.item != "tconstruct:materials_and_you" { return false; }
    if remote || game.lan_acting.is_none() {
        let path = game.options.resources.join("assets/tconstruct/materials-and-you.zip");
        let Some(pages) = source_pages(game, &path) else { return false; };
        game.open_book(BookScreen::View { pages, page: 0, lectern: None });
        if remote {
            let hand = u8::from(game.hand == Hand::Off);
            game.send(S::CustomPayload(horizon_protocol::packets::configuration::CustomPayload {
                channel: "tconstruct:materials_book".into(), data: vec![hand],
            }));
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::split_page;
    #[test]
    fn split_keeps_unicode_and_words() {
        assert_eq!(split_page("one two 猫 three", 8), vec!["one two", "猫 three"]);
    }
}
