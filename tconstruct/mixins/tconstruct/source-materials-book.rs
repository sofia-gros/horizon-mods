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

fn source_pages(path: &std::path::Path) -> Option<Vec<String>> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).ok()?).ok()?;
    let root = "assets/tconstruct/book/materials_and_you/";
    let index: serde_json::Value = serde_json::from_str(&read(&mut zip, &format!("{root}index.json"))?).ok()?;
    let language = read(&mut zip, &format!("{root}en_us/language.lang"))?;
    let labels: std::collections::HashMap<_,_> = language.lines().filter_map(|line| line.split_once('=')).collect();
    let mut pages = Vec::new();
    for chapter in index.as_array()? {
        let Some(data) = chapter.get("data").and_then(|x| x.as_str()).filter(|s| *s != "no-load") else { continue; };
        let key = chapter.get("name").and_then(|x| x.as_str())?;
        let heading = labels.get(key).copied().unwrap_or(key);
        let Some(section) = read(&mut zip, &format!("{root}{data}")) else { continue; };
        let Ok(section) = serde_json::from_str::<serde_json::Value>(&section) else { continue; };
        let mut chapter_pages = Vec::new();
        for page in section.as_array()? {
            let Some(relative) = page.get("data").and_then(|x| x.as_str()) else { continue; };
            let Some(raw) = read(&mut zip, &format!("{root}en_us/{relative}")) else { continue; };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else { continue; };
            let title = value.get("title").and_then(|x| x.as_str()).unwrap_or("");
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
                chapter_pages.push(if title.is_empty() { part } else { format!("{title}\n\n{part}") });
            }
        }
        if !chapter_pages.is_empty() { pages.push(heading.to_owned()); pages.extend(chapter_pages); }
    }
    (!pages.is_empty()).then_some(pages)
}

pub(super) fn open(game: &mut Game<'_>, stack: &ItemStack, remote: bool) -> bool {
    if &*stack.item != "tconstruct:materials_and_you" { return false; }
    if remote || game.lan_acting.is_none() {
        let path = game.options.resources.join("assets/tconstruct/materials-and-you.zip");
        let Some(pages) = source_pages(&path) else { return false; };
        game.open_book(BookScreen::View { pages: pages.into_iter().map(Text::literal).collect(), page: 0, lectern: None });
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
