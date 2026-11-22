//! The six books (TinkerBookItem and the client book definitions of TinkerBook, SlimeKnights, MIT; see LICENSE): unstackable items
//! whose use opens the book of the same name. The book data is the original's Mantle data, shipped under `assets/tconstruct/book`:
//! a book's `index.json` names its sections, each section file lists its pages, and each page's JSON sits in a folder per language.
//! The books are built from the active client resources when they reload (the initial notification included) and registered with
//! PlutoLib's book registry; using a book item asks PlutoLib to open the registered book, which draws and turns its pages.
//!
//! What the original builds at load that this port does not: Mantle's section transformers generate pages from the loaded
//! registries — a page per material for the material sections (`TierRangeMaterialSectionTransformer`, the sections whose index
//! data is `no-load`), a page per tool and per modifier behind the `tconstruct:load_tools` / `tconstruct:load_modifiers` group
//! anchors of the section files, the armor section's tool pages, and the encyclopedia's fluid effect pages
//! (`FluidEffectInjectingTransformer`). Those page kinds render registry tables, which PlutoLib's book renderer does not draw:
//! its decoded kinds are text, image_text and crafting, and any other kind is kept as an opaque page. So a generated section is
//! present with its name, its icon and one index page carrying its title and subtext (the shape of the `mantle:index` page the
//! original itself puts first in a section), and a group anchor likewise leaves one index page titled with the group's name;
//! the generated pages themselves are the gap. Every file-backed page is carried over in full (the one crafting page that
//! spells out a literal grid instead of naming a recipe rides as an opaque page, as the service's crafting pages reference
//! recipes). Mantle's padding transformer
//! (blank pages so a section starts on a left page of a two-page spread) has no counterpart in the single-page view: the
//! `mantle:blank` and `mantle:right_padding` entries of the section files become blank pages where they sit. The one section
//! condition the original ships (materials and you's armor section, shown when its book armor tag is not empty) is not
//! evaluated: the section is always present, as its tag is not empty in the original's own data either.
use pluto_lib::{ActionResult, Book, BookAppearance, BookIcon, BookPage, BookRegistry, BookSection, I18n, Identifier, Nbt, NbtCompound, ResourceManagerHelper, ResourceType, UseItemCallback, open_book};
use serde_json::Value;

/// The book items, in TinkerCommons' registration order; each opens the book of the same name (TinkerBookIDs).
pub(crate) const BOOKS: [&str; 6] = ["materials_and_you", "puny_smelting", "mighty_smelting", "tinkers_gadgetry", "fantastic_foundry", "encyclopedia"];

/// A resource reader: the bytes of `book/...` path in the tconstruct namespace, or None when it is not there. The game reads
/// the active client resources; a test reads the shipped content.
pub(crate) type Read<'a> = dyn Fn(&str) -> Option<Vec<u8>> + 'a;

fn read_json(read: &Read<'_>, path: &str) -> Option<Value> {
    serde_json::from_slice(&read(path)?).ok()
}

/// One book's section titles and subtexts (its per-language `language.lang`): the reader's language over the English table,
/// key by key, as Mantle falls back per file for the pages themselves.
fn language_table(read: &Read<'_>, book: &str, language: &str) -> Vec<(String, String)> {
    let mut table: Vec<(String, String)> = Vec::new();
    for language in ["en_us", language] {
        let Some(bytes) = read(&format!("book/{book}/{language}/language.lang")) else { continue };
        for line in String::from_utf8_lossy(&bytes).lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            let (key, value) = (key.trim(), value.trim());
            if key.is_empty() { continue; }
            if let Some(entry) = table.iter_mut().find(|(known, _)| known == key) { entry.1 = value.to_owned(); }
            else { table.push((key.to_owned(), value.to_owned())); }
        }
    }
    table
}

fn lookup<'a>(table: &'a [(String, String)], key: &str) -> Option<&'a str> {
    table.iter().find(|(known, _)| known == key).map(|(_, value)| value.as_str())
}

/// A section's icon: the index writes either the flat form PlutoLib decodes (`{"item": id, "nbt": ...}`) or an ingredient
/// object (`{"item": {"type": "forge:nbt", "item": id, "nbt": {...}}}`), which is flattened to the same form first.
fn icon(value: &Value) -> Option<BookIcon> {
    let item = value.get("item")?;
    let flat;
    let value = if item.is_object() {
        let mut object = serde_json::Map::new();
        object.insert("item".to_owned(), item.get("item")?.clone());
        if let Some(nbt) = item.get("nbt") { object.insert("nbt".to_owned(), nbt.clone()); }
        flat = Value::Object(object);
        &flat
    } else { value };
    BookIcon::from_json(value).ok()
}

/// The index page of a generated section or group: an opaque `mantle:index` page with the title (and subtext) the original
/// gives it, the page kind the original itself uses for a section's first page.
fn index_page(title: &str, subtext: Option<&str>) -> BookPage {
    let mut data: NbtCompound = [("title".to_owned(), Nbt::String(title.to_owned()))].into_iter().collect();
    if let Some(subtext) = subtext { data.insert("subText".to_owned(), Nbt::String(subtext.to_owned())); }
    BookPage::Opaque { kind: "mantle:index".to_owned(), data }
}

/// One page entry of a section file, read tolerantly: the shipped files also carry the transformer's anchors (a `no-load`
/// data with a load extraData), blank pages and padding markers, which PlutoLib's strict `section_page_refs` refuses.
struct PageRef { name: Option<String>, kind: Option<String>, data: Option<String>, extra: Option<String> }

fn page_ref(entry: &Value) -> PageRef {
    let text = |key: &str| entry.get(key).and_then(Value::as_str).map(str::to_owned);
    let extra = entry.get("extraData").and_then(Value::as_object)
        .and_then(|extra| extra.keys().find(|key| key.starts_with("tconstruct:load_")).cloned());
    PageRef { name: text("name"), kind: text("type"), data: text("data"), extra }
}

/// Builds one book from its Mantle data in the reader's language. A page whose file is in neither the reader's language nor
/// English is left out (Mantle logs and skips it too); a section left with no pages is left out, as the registry takes none.
pub(crate) fn build_book(read: &Read<'_>, name: &str, language: &str) -> Result<Book, String> {
    let root = format!("book/{name}");
    let index = read_json(read, &format!("{root}/index.json")).ok_or_else(|| format!("book {name} has no index"))?;
    let entries = index.as_array().ok_or_else(|| format!("book {name}'s index is not a section list"))?;
    let appearance = read_json(read, &format!("{root}/{language}/appearance.json"))
        .or_else(|| read_json(read, &format!("{root}/appearance.json")))
        .map(|data| BookAppearance::from_json(&data)).transpose()?.unwrap_or_default();
    let table = language_table(read, name, language);
    let mut sections = Vec::new();
    for entry in entries {
        let Some(section_name) = entry.get("name").and_then(Value::as_str) else { continue };
        let title = lookup(&table, section_name).unwrap_or(section_name).to_owned();
        let icon = entry.get("icon").and_then(icon);
        let data = entry.get("data").and_then(Value::as_str);
        if data.is_none() || data == Some("no-load") {
            // A transformer-generated section: its title page stands where the generated pages would follow.
            let subtext = lookup(&table, &format!("{section_name}.subtext")).map(str::to_owned);
            let page = index_page(&title, subtext.as_deref());
            sections.push(BookSection { name: title, icon, pages: vec![page] });
            continue;
        }
        let Some(refs) = read_json(read, &format!("{root}/{}", data.unwrap())).and_then(|file| file.as_array().cloned()) else {
            eprintln!("Tinkers: book {name}'s section {section_name} does not load");
            continue;
        };
        let mut pages = Vec::new();
        for entry in &refs {
            let page_ref = page_ref(entry);
            match (page_ref.kind.as_deref(), page_ref.data.as_deref()) {
                // A blank page, or a padding marker: a blank page where it sits.
                (Some("mantle:blank"), _) | (Some("mantle:right_padding"), _) => pages.push(BookPage::Text { title: None, text: Vec::new() }),
                (Some(kind), Some(path)) if path != "no-load" => {
                    let page = read_json(read, &format!("{root}/{language}/{path}"))
                        .or_else(|| read_json(read, &format!("{root}/en_us/{path}")));
                    match page.map(|data| BookPage::from_json(kind, &data)) {
                        Some(Ok(page)) => pages.push(page),
                        Some(Err(error)) => eprintln!("Tinkers: book {name}'s page {path} does not decode: {error}"),
                        None => eprintln!("Tinkers: book {name}'s page {path} is not in the resources"),
                    }
                }
                // A group anchor (its data is `no-load` or absent, with a load extraData): the generated tool or
                // modifier pages are the gap; the group's title page marks the group.
                _ if page_ref.name.is_some() => {
                    let group = page_ref.name.as_deref().unwrap_or_default();
                    let title = lookup(&table, &format!("{section_name}.{group}")).unwrap_or(group).to_owned();
                    pages.push(index_page(&title, None));
                }
                _ => eprintln!("Tinkers: book {name} has a page entry it cannot read: {entry}"),
            }
        }
        if pages.is_empty() { eprintln!("Tinkers: book {name}'s section {section_name} has no pages and is left out"); continue; }
        sections.push(BookSection { name: title, icon, pages });
    }
    let id = Identifier::new("tconstruct", name).map_err(|error| error.to_string())?;
    Ok(Book { id, appearance, sections })
}

/// Builds every book from the active client resources and registers the ones not already registered (a resource reload
/// rebuilds nothing: the registry keeps the first build, so a language change does not retitle an open book mid-read).
fn load_books() {
    let read = |path: &str| {
        let id = Identifier::new("tconstruct", path).ok()?;
        ResourceManagerHelper::read_client_resource(&id)
    };
    let language = I18n::language();
    for name in BOOKS {
        let id = Identifier::new("tconstruct", name).expect("a book name is an identifier path");
        if BookRegistry::get(&id).is_some() { continue; }
        match build_book(&read, name, &language) {
            Ok(book) => if let Err(error) = BookRegistry::register(book) { eprintln!("Tinkers: book {name} not registered: {error}"); },
            Err(error) => eprintln!("Tinkers: {error}"),
        }
    }
}

/// Registers the book opening and the book building. Called once from the product's hooks.
pub(crate) fn register() {
    // TinkerBookItem: using the item opens its book (AbstractBookItem.use). Opening is a client act; anywhere else
    // PlutoLib answers false and the use passes to the game untouched.
    UseItemCallback::EVENT.register(|context| {
        let Some(name) = context.item.strip_prefix("tconstruct:") else { return ActionResult::Pass };
        if !BOOKS.contains(&name) { return ActionResult::Pass; }
        let Ok(id) = Identifier::new("tconstruct", name) else { return ActionResult::Pass };
        if open_book(&id) { ActionResult::Success } else { ActionResult::Pass }
    });
    ResourceManagerHelper::get(ResourceType::ClientResources)
        .register_reload_listener(Identifier::new("tconstruct", "books").expect("valid listener id"), load_books);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    /// An in-memory book folder: `materials_and_you` reduced to one file section, one generated section and one group anchor.
    fn fixture() -> HashMap<String, Vec<u8>> {
        let mut files = HashMap::new();
        let mut put = |path: &str, value: Value| { files.insert(format!("book/materials_and_you/{path}"), serde_json::to_vec(&value).unwrap()); };
        put("index.json", json!([
            {"name": "intro", "data": "sections/introduction.json", "icon": {"item": "tconstruct:pattern"}},
            {"name": "materials", "data": "no-load", "icon": {"item": {"type": "forge:nbt", "item": "tconstruct:tool_binding", "nbt": {"Material": "tconstruct:rock#stone"}}}},
            {"name": "tools", "data": "sections/tools.json", "icon": {"item": "minecraft:carved_pumpkin"}}
        ]));
        put("appearance.json", json!({"coverColor": -1, "centerPageTitles": true, "exportTitle": "Materials and You"}));
        put("sections/introduction.json", json!([
            {"name": "introduction", "type": "mantle:text", "data": "intro/welcome.json"},
            {"name": "craft", "type": "mantle:crafting", "data": "intro/pattern_craft.json"},
            {"type": "mantle:blank"}
        ]));
        put("en_us/intro/welcome.json", json!({"title": "Materials & You", "text": [{"text": "Welcome."}]}));
        put("en_us/intro/pattern_craft.json", json!({"title": "Pattern", "recipe": "tconstruct:tables/pattern", "grid_size": "large"}));
        put("sections/tools.json", json!([
            {"name": "group_general", "type": "mantle:text", "data": "no-load", "extraData": {"tconstruct:load_tools": {"tag": "tconstruct:modifiable/small"}}},
            {"name": "repairing", "type": "mantle:text", "data": "tools/repairing.json"}
        ]));
        put("en_us/tools/repairing.json", json!({"title": "Repairing", "text": [{"text": "Fix it."}]}));
        files.insert("book/materials_and_you/en_us/language.lang".to_owned(),
            b"intro=Introduction\nmaterials=Tier 1 Materials\nmaterials.subtext=Make parts from these.\ntools=Tools\ntools.group_general=General\n".to_vec());
        files
    }

    #[test]
    fn a_book_builds_with_its_file_pages_generated_sections_and_group_anchors() {
        let files = fixture();
        let read = |path: &str| files.get(path).cloned();
        let book = build_book(&read, "materials_and_you", "en_us").unwrap();
        assert_eq!(book.id.to_string(), "tconstruct:materials_and_you");
        assert!(book.appearance.center_page_titles);
        assert_eq!(book.appearance.export_title.as_deref(), Some("Materials and You"));
        assert_eq!(book.sections.len(), 3);
        let intro = &book.sections[0];
        assert_eq!(intro.name, "Introduction", "the section list shows the language table's title");
        assert_eq!(intro.icon.as_ref().map(|icon| icon.item().to_string()), Some("tconstruct:pattern".to_owned()));
        assert_eq!(intro.pages.len(), 3, "two file pages and the blank page");
        assert_eq!(intro.pages[0].title(), Some("Materials & You"));
        assert_eq!(intro.pages[1].kind(), "mantle:crafting");
        assert_eq!(intro.pages[2], BookPage::Text { title: None, text: Vec::new() }, "a blank entry is a blank page");
        let materials = &book.sections[1];
        assert_eq!(materials.name, "Tier 1 Materials");
        assert_eq!(materials.icon.as_ref().map(|icon| icon.item().to_string()), Some("tconstruct:tool_binding".to_owned()),
            "the ingredient-shaped icon flattens to its item");
        assert_eq!(materials.pages.len(), 1, "a generated section keeps its index page only");
        assert_eq!(materials.pages[0].kind(), "mantle:index");
        assert_eq!(materials.pages[0].title(), Some("Tier 1 Materials"));
        let tools = &book.sections[2];
        assert_eq!(tools.pages.len(), 2, "the group anchor's index page and the file page");
        assert_eq!(tools.pages[0].title(), Some("General"), "the anchor is titled from the language table");
        assert_eq!(tools.pages[1].title(), Some("Repairing"));
        assert_eq!(book.page_count(), 6);
    }

    #[test]
    fn a_page_falls_back_to_english_and_a_missing_page_is_left_out() {
        let mut files = fixture();
        files.remove("book/materials_and_you/en_us/tools/repairing.json");
        let read = |path: &str| files.get(path).cloned();
        let book = build_book(&read, "materials_and_you", "ja_jp").unwrap();
        assert_eq!(book.sections[0].pages.len(), 3, "the English pages stand in for the missing translation");
        assert_eq!(book.sections[2].pages.len(), 1, "only the group anchor remains of the tools section");
    }
}
