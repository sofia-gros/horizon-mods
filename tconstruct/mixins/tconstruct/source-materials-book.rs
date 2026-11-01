// Source text pages in Horizon's existing book screen. Source image and
// dynamic recipe/material pages remain in the archive for future rendering.
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

fn dynamic_materials(game: &Game<'_>, key: &str) -> Option<Vec<String>> {
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
    selected.sort_by_key(|(id, (_, _, tier, order, _))| (*tier, *order, (*id).clone()));
    Some(selected.into_iter().map(|(id, _)| {
        let (namespace, name) = id.split_once(':').unwrap_or(("tconstruct", id));
        game.translate(&format!("material.{namespace}.{name}"), name)
    }).collect())
}

fn source_pages(game: &Game<'_>, path: &std::path::Path) -> Option<Vec<Text>> {
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
            if let Some(materials) = dynamic_materials(game, key) {
                let mut chapter_pages = Vec::new();
                let subtext = labels.get(format!("{key}.subtext").as_str()).copied().unwrap_or("");
                if !subtext.is_empty() { chapter_pages.extend(split_page(subtext, 120).into_iter().map(Text::literal)); }
                for names in materials.chunks(8) { chapter_pages.push(Text::literal(names.join("\n"))); }
                if !chapter_pages.is_empty() { chapters.push((heading.to_owned(), chapter_pages)); }
            }
            continue;
        }
        let Some(section) = read(&mut zip, &format!("{root}{data}")) else { continue; };
        let Ok(section) = serde_json::from_str::<serde_json::Value>(&section) else { continue; };
        let mut chapter_pages = Vec::new();
        for page in section.as_array()? {
            let Some(relative) = page.get("data").and_then(|x| x.as_str()) else { continue; };
            let Some(raw) = read(&mut zip, &format!("{root}en_us/{relative}")) else { continue; };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else { continue; };
            let title = value.get("title").and_then(|x| x.as_str()).unwrap_or("");
            let graphic = value.get("image").and_then(|v| v.get("file")).and_then(|v| v.as_str())
                .map(|s| serde_json::json!({"image":s}))
                .or_else(|| value.get("recipe").and_then(|v| v.as_str()).map(|s| serde_json::json!({"recipe":s})));
            if let Some(graphic) = graphic {
                chapter_pages.push(Text::styled(title, horizon_client::text::Style {
                    insertion: Some(format!("tconstruct:book:{}", graphic)), ..Default::default()
                }));
            }
            let mut text = String::new();
            for key in ["text", "description"] {
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
    let mut contents = Text::literal("Materials & You\n\n");
    let mut target = 2i32;
    let mut pages = Vec::new();
    for (heading, chapter) in chapters {
        let link = Text::styled(format!("{heading}\n"), horizon_client::text::Style {
            click: Some(horizon_client::text::ClickEvent::ChangePage(target)),
            underlined: Some(true), ..Default::default()
        });
        contents.push(link);
        pages.push(Text::literal(heading));
        pages.extend(chapter);
        target = i32::try_from(pages.len() + 2).ok()?;
    }
    pages.insert(0, contents);
    Some(pages)
}

pub(super) fn open(game: &mut Game<'_>, stack: &ItemStack, remote: bool) -> bool {
    if &*stack.item != "tconstruct:materials_and_you" { return false; }
    if remote || game.lan_acting.is_none() {
        let path = game.options.resources.join("assets/tconstruct/materials-and-you.zip");
        let Some(pages) = source_pages(game, &path) else { return false; };
        game.open_book(BookScreen::View { pages, page: 0, lectern: None });
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
