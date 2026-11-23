//! The six books (TinkerBookItem and the client book definitions of TinkerBook, SlimeKnights, MIT; see LICENSE): unstackable items
//! whose use opens the book of the same name. The book data is the original's Mantle data, shipped under `assets/tconstruct/book`:
//! a book's `index.json` names its sections, each section file lists its pages, and each page's JSON sits in a folder per language.
//! The books are built from the active client resources when they reload (the initial notification included) and registered with
//! PlutoLib's book registry; using a book item asks PlutoLib to open the registered book, which draws and turns its pages.
//!
//! The generated content Mantle's section transformers produce in the original is built here too, as PlutoLib table pages
//! (one row per entry, keeping the generated content's own page kind): the tag injectors put a page per tool and per modifier
//! behind the `tconstruct:load_tools` / `tconstruct:load_modifiers` anchors of the section files and of the armor sections,
//! the grouping transformers then index those sections with a listing page of group headers and page links, the tier range
//! transformer fills each `tconstruct:material_tier` section with an index of its materials and a stat page per material
//! (from the live material registry, so a datapack's materials appear like the original's), and the encyclopedia's fluid
//! effect transformer does the same for the fluid effect entries. The books are rebuilt and replace their registrations
//! when the materials load, so a first build made before the registries arrive is superseded by the full one; a book that
//! is open keeps the copy it opened with. The one section condition the original ships (an armor section, shown when its
//! book armor tag is not empty) is evaluated against the captured item registry when there is one.
//!
//! What still differs from the original, and why: Mantle's padding transformer (blank pages so a section starts on a left
//! page of a two-page spread) has no counterpart in the single-page view — the `mantle:blank` and `mantle:right_padding`
//! entries of the section files become blank pages where they sit. A file section's own `mantle:index` page becomes a
//! listing of the section's pages (the index transformer below); no index page is added to a file section that does not
//! carry one, and the generated sections lead with their index page, the shape of the original's own first page there.
//! A generated page's sidebar of display items rides its first page: a material's parts in that material, a tool page's
//! tool and parts, a fluid page's buckets. A modifier page's sidebar (its recipe's items) is not drawn: the modifier
//! definitions carry no recipe, and the modifier recipes are not among the snapshots the client holds. A crafting
//! page's grid fills from the crafting recipes captured at the server's data reload, so it draws in full where that
//! capture ran (a local world); a reader on a remote server sees the grid the page decoded with, which is empty.
//! Sort-free tag injections (the armor and staff sections) take their order from the captured registry snapshot, whose
//! tag members are a set, where the original walks the tag file's own order. A damage effect's book description names
//! its damage type through the type's message id, which no registry this port keeps carries, so those lines use the
//! unsuffixed translation. The fluid transformer keeps the original's early return: an entry whose ingredient names no
//! fluids ends the section's generation, as it does there.
use std::sync::Arc;

use pluto_lib::{ActionResult, Book, BookAppearance, BookIcon, BookPage, BookRegistry, BookRow, BookStructure, I18n, Identifier, Nbt, NbtCompound, RegistryLookup, ResourceManagerHelper, ResourceType, UseItemCallback, open_book};
use serde_json::Value;

use crate::commands;
use crate::fluid_effects::{BlockEffect, EntityEffect, FluidEffects, GeneralEffect, MobEffectDef, TimeAction};
use crate::fluid_ingredient::Source;
use crate::material_stats::Stats;
use crate::materials::Definitions;
use crate::modifiers::Modifier;

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
/// gives it, the page kind the original itself uses for a section's first page. It is what a generated section keeps when
/// the registries its pages come from have not loaded, so the section is present with its name and icon.
fn index_page(title: &str, subtext: Option<&str>) -> BookPage {
    let mut data: NbtCompound = [("title".to_owned(), Nbt::String(title.to_owned()))].into_iter().collect();
    if let Some(subtext) = subtext { data.insert("subText".to_owned(), Nbt::String(subtext.to_owned())); }
    BookPage::Opaque { kind: "mantle:index".to_owned(), data }
}

/// The translation of `key` in the language in use.
fn tr(key: &str) -> String { I18n::translate(key) }

/// `key`'s translation with its arguments substituted (the original's translatable components): `%s` and `%d`
/// take the next argument, `%1$s` the numbered one, `%%` a percent. PlutoLib's own argument formatting does
/// not know `%d`, which the fluid effect texts use, so the substitution lives here.
fn tr_args(key: &str, args: &[&str]) -> String {
    let template = I18n::translate(key);
    let mut out = String::new();
    let mut next = 0;
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' { out.push(c); continue; }
        match chars.peek().copied() {
            Some('%') => { chars.next(); out.push('%'); }
            Some(kind @ ('s' | 'd' | 'f')) => {
                chars.next();
                let _ = kind;
                out.push_str(args.get(next).copied().unwrap_or(""));
                next += 1;
            }
            Some(_) => {
                // A numbered argument, %2$s and kin: read the digits, then the $ and the kind letter.
                let mut probe = chars.clone();
                let mut digits = String::new();
                while let Some(&d) = probe.peek() {
                    if d.is_ascii_digit() { digits.push(d); probe.next(); } else { break; }
                }
                if probe.next() == Some('$') && matches!(probe.peek(), Some('s' | 'd' | 'f')) {
                    probe.next();
                    chars = probe;
                    let index: usize = digits.parse().unwrap_or(0);
                    out.push_str(args.get(index.wrapping_sub(1)).copied().unwrap_or(""));
                } else {
                    out.push('%');
                }
            }
            None => out.push('%'),
        }
    }
    out
}

/// A float as the original's component arguments print it (Java's Float.toString: `2.0`, `0.5`).
fn float_text(value: f32) -> String { format!("{value:?}") }

/* The registry data the transformers generate from */

/// The loaded registries a book build generates its pages from, gathered once per build round. Any part may
/// be missing (a build before the data arrives, or a test's empty world); the transformers then generate
/// nothing for it and the sections keep their index pages, as a book built at first resource load does.
#[derive(Clone, Default)]
pub(crate) struct GenData {
    /// The client material definitions (the server's own in a local world, the synced snapshot in a remote one).
    materials: Option<Definitions>,
    /// The modifier registry, tags included.
    modifiers: Option<crate::modifiers::Registry>,
    /// The captured item registry: which items exist and what each item tag holds.
    items: Option<Arc<RegistryLookup>>,
    /// The captured fluid registry, for resolving fluid tags to their fluids.
    fluids: Option<Arc<RegistryLookup>>,
    /// The fluid effect entries, in load order.
    fluid_effects: Option<Vec<(Identifier, FluidEffects)>>,
}

/// Gathers the current registries for a book build.
fn gather_gen_data() -> GenData {
    let materials = crate::common::with_client_materials(|definitions| definitions.cloned());
    let modifiers = crate::common::server_modifiers().map(|registry| registry.read().expect("modifier registry lock poisoned").clone());
    let fluid_effects = crate::common::fluid_effects().map(|data| {
        let data = data.read().expect("fluid effects lock poisoned");
        data.effects.entries().map(|(id, effects)| (id.clone(), effects.clone())).collect()
    });
    GenData { materials, modifiers, items: crate::common::item_snapshot(), fluids: crate::common::fluid_snapshot(), fluid_effects }
}

/* The raw book: sections and pages before the transformers run */

/// What a page entry turned out to be, for the rules that look at content rather than text: the grouping
/// transformers drop a tool page whose item does not exist and a modifier page whose modifier is the
/// registry's default, however the page was loaded.
#[derive(Clone, Debug)]
enum GenPage {
    Tool(Identifier),
    Modifier(Identifier),
}

/// One page entry of a section file (or an injected page), before the transformers run: its entry fields,
/// the pages it decoded or generated to (usually one; a generated content that overflows one page makes
/// several), and what generated content it is, if any.
struct RawPage {
    name: Option<String>,
    kind: Option<String>,
    data: Option<String>,
    extra: serde_json::Map<String, Value>,
    pages: Vec<BookPage>,
    content: Option<GenPage>,
}

impl RawPage {
    fn new(entry: &Value) -> Self {
        let text = |key: &str| entry.get(key).and_then(Value::as_str).map(str::to_owned);
        Self {
            name: text("name"),
            kind: text("type"),
            data: text("data"),
            extra: entry.get("extraData").and_then(Value::as_object).cloned().unwrap_or_default(),
            pages: Vec::new(),
            content: None,
        }
    }

    /// The page's title for an index or listing: its first page's title, if it has one.
    fn title(&self) -> Option<&str> {
        self.pages.first().and_then(|page| page.title())
    }
}

/// One section of a book before the transformers run.
struct RawSection {
    /// The index entry's name (`tools`, `melee_harvest_materials`), which the transformers match on.
    name: String,
    /// The section's title from the language table.
    title: String,
    icon: Option<BookIcon>,
    extra: serde_json::Map<String, Value>,
    /// Whether the section's pages are transformer-generated (its index data is absent or `no-load`):
    /// such a section keeps an index page when nothing generates, where a file section left with no
    /// pages is left out, as the registry takes no empty section.
    generated: bool,
    pages: Vec<RawPage>,
}

/* Table page assembly */

/// How tall a table page's body may grow, in the renderer's pixels: the panel is 248 tall, the content
/// keeps 10 at the top and 16 at the foot and a title takes 14, leaving 208; pages fill to 196 so the
/// renderer's own clip never cuts a row the producer counted.
const PAGE_BUDGET: f32 = 196.0;
/// A table row's height in the renderer.
const ROW_HEIGHT: f32 = 12.0;
/// A wrapped text line's height in the renderer.
const LINE_HEIGHT: f32 = 10.0;
/// How many characters of text fill one rendered line at a book page's content width (218 with the rail).
const CHARS_PER_LINE: usize = 36;

/// The height a block of text runs takes when wrapped, estimated by characters (the producer has no
/// canvas to measure with; the estimate is what the pagination splits by).
fn text_height(text: &[pluto_lib::BookText]) -> f32 {
    text.iter().map(|run| {
        let lines = (run.text.chars().count() / CHARS_PER_LINE + 1) as f32;
        lines * LINE_HEIGHT + if run.paragraph { LINE_HEIGHT } else { 0.0 }
    }).sum()
}

/// Splits one generated content into table pages: the intro rides the first page, the tail text the last,
/// and the rows fill each page to the budget — the counterpart of the original's contents flowing over a
/// two-page spread. Every page keeps the content's title and kind.
fn paginate(kind: &str, title: Option<String>, intro: Vec<pluto_lib::BookText>, rows: Vec<BookRow>, text: Vec<pluto_lib::BookText>) -> Vec<BookPage> {
    let mut pages = Vec::new();
    let mut rest = rows.as_slice();
    let mut first = true;
    loop {
        let mut budget = PAGE_BUDGET;
        let page_intro = if first { budget -= text_height(&intro); intro.clone() } else { Vec::new() };
        let mut count = 0;
        while count < rest.len() && ROW_HEIGHT * (count + 1) as f32 <= budget { count += 1; }
        if count == 0 && !rest.is_empty() { count = 1; }
        let chunk: Vec<BookRow> = rest[..count].to_vec();
        rest = &rest[count..];
        let last = rest.is_empty();
        // The tail text rides the last page when it fits under its rows, else a page of its own.
        let (page_text, spill) = if last && (chunk.len() as f32 * ROW_HEIGHT + text_height(&text) <= PAGE_BUDGET || chunk.is_empty()) {
            (text.clone(), false)
        } else if last {
            (Vec::new(), true)
        } else {
            (Vec::new(), false)
        };
        pages.push(BookPage::Table { kind: kind.to_owned(), title: title.clone(), intro: page_intro, rows: chunk, text: page_text, sidebar: Vec::new() });
        first = false;
        if last {
            if spill { pages.push(BookPage::Table { kind: kind.to_owned(), title: title.clone(), intro: Vec::new(), rows: Vec::new(), text: text.clone(), sidebar: Vec::new() }); }
            break;
        }
    }
    pages
}

/// Gives the first of `pages` a sidebar of display items (a generated page's sidebar in the
/// original: a material's built tools, a tool's recipe parts, a modifier's recipe, a fluid's
/// bucket), drawn in a column at the page's right edge.
fn with_sidebar(mut pages: Vec<BookPage>, items: Vec<pluto_lib::ItemSlot>) -> Vec<BookPage> {
    if let Some(BookPage::Table { sidebar, .. }) = pages.first_mut() { *sidebar = items; }
    pages
}

/// Adds `by` to every table row link in the pages (links are written section-locally by the transformers
/// and shifted to the book's flat indices once the sections before them are final).
fn shift_links(pages: &mut [BookPage], by: usize) {
    if by == 0 { return; }
    for page in pages {
        if let BookPage::Table { rows, .. } = page {
            for row in rows {
                if let BookRow::Entry { link: Some(link), .. } = row { *link += by; }
            }
        }
    }
}

/// One plain table row.
fn book_row(icon: Option<BookIcon>, label: String, value: Option<String>, link: Option<usize>) -> BookRow {
    BookRow::Entry { icon, label, value, link }
}

/// A page's `text` list: entries of `{text, paragraph?}`; a bare string entry is one plain run (the same
/// reading PlutoLib's own page decoding does, for the generated contents' own files).
fn text_runs(value: Option<&Value>) -> Vec<pluto_lib::BookText> {
    if let Some(Value::String(text)) = value { return vec![pluto_lib::BookText::plain(text)]; }
    let Some(Value::Array(entries)) = value else { return Vec::new() };
    entries.iter().filter_map(|entry| match entry {
        Value::String(text) => Some(pluto_lib::BookText::plain(text)),
        Value::Object(_) => entry.get("text").and_then(Value::as_str).map(|text| pluto_lib::BookText {
            text: text.to_owned(),
            paragraph: entry.get("paragraph").and_then(Value::as_bool).unwrap_or(false),
        }),
        _ => None,
    }).collect()
}

/// A page's string list (`properties`, `effects`, a fluid page's effect lists).
fn string_list(value: Option<&Value>) -> Vec<String> {
    let Some(Value::Array(entries)) = value else { return Vec::new() };
    entries.iter().filter_map(|entry| entry.as_str().map(str::to_owned)).collect()
}

/// An item's display name: its description id's translation. An item's description id is its `item.` key,
/// a block item's its `block.` key; the one that has a translation wins, as the game's own lookup falls
/// out for the items the books name.
fn item_name(item: &Identifier) -> String {
    let item_key = format!("item.{}.{}", item.namespace(), item.path());
    if I18n::has(&item_key) { return tr(&item_key); }
    let block_key = format!("block.{}.{}", item.namespace(), item.path());
    if I18n::has(&block_key) { return tr(&block_key); }
    tr(&item_key)
}

/// An item's description id (the key its `.description` text hangs off), by the same choice as its name.
fn description_id(item: &Identifier) -> String {
    let item_key = format!("item.{}.{}", item.namespace(), item.path());
    if I18n::has(&item_key) { return item_key; }
    let block_key = format!("block.{}.{}", item.namespace(), item.path());
    if I18n::has(&block_key) { return block_key; }
    item_key
}

/// A modifier's translation key: its own when the definition names one, else `modifier.<id>`.
fn modifier_key(modifier: &Modifier) -> String {
    modifier.translation_key.clone().unwrap_or_else(|| format!("modifier.{}.{}", modifier.id.namespace(), modifier.id.path()))
}

/// A modifier's display name (the plain name, no level: the form the book's titles and trait lines use).
fn modifier_name(modifier: &Modifier) -> String { tr(&modifier_key(modifier)) }

/// The modifier registry's default value: the empty modifier every lookup of an unknown id lands on.
fn is_default_modifier(modifiers: &crate::modifiers::Registry, id: &Identifier) -> bool {
    id.to_string() == "tconstruct:empty" || modifiers.get(id).is_none()
}

/* Tool and modifier pages (ContentTool / ContentModifier) */

/// Builds a tool page's content: the tool item's name as title, the page's text, and its properties under
/// their header, bulleted as the original lists them. Without a data file the text is the item's own
/// description line and there are no properties (ContentTool's fallback content for an injected page).
fn tool_pages(item: &Identifier, json: Option<&Value>) -> Vec<BookPage> {
    let (text, properties) = match json {
        Some(json) => (text_runs(json.get("text")), string_list(json.get("properties"))),
        None => (vec![pluto_lib::BookText::plain(&tr(&format!("{}.description", description_id(item))))], Vec::new()),
    };
    let mut rows = Vec::new();
    if !properties.is_empty() {
        rows.push(BookRow::Header { icon: None, text: tr("book.tconstruct.tool.properties") });
        rows.extend(properties.into_iter().map(|property| book_row(None, format!("● {property}"), None, None)));
    }
    // The sidebar: the tool itself, then its recipe parts (plain part stacks; the definition's
    // part list, when the definitions are at hand where the books build).
    let mut sidebar = vec![pluto_lib::ItemSlot::new(item.clone(), 1)];
    if let Ok(definitions) = crate::tool_definitions::register().read() {
        if let Some(definition) = definitions.data.get(item) {
            sidebar.extend(definition.tool_parts().iter().map(|part| pluto_lib::ItemSlot::new(part.clone(), 1)));
        }
    }
    with_sidebar(paginate("tconstruct:tool", Some(item_name(item)), text, rows, Vec::new()), sidebar)
}

/// Builds a modifier page's content: the modifier's name as title, the page's text, and its effects under
/// their header. Without a data file the text is the modifier's own description (ContentModifier's
/// fallback). None when the modifier does not resolve, the case the original drops in a grouped section.
fn modifier_pages(modifiers: &crate::modifiers::Registry, id: &Identifier, json: Option<&Value>) -> Option<Vec<BookPage>> {
    if is_default_modifier(modifiers, id) { return None; }
    let modifier = modifiers.get(id)?;
    let (text, effects) = match json {
        Some(json) => (text_runs(json.get("text")), string_list(json.get("effects"))),
        None => (vec![pluto_lib::BookText::plain(&tr(&format!("{}.description", modifier_key(modifier))))], Vec::new()),
    };
    let mut rows = Vec::new();
    if !effects.is_empty() {
        rows.push(BookRow::Header { icon: None, text: tr("book.tconstruct.modifiers.effect") });
        rows.extend(effects.into_iter().map(|effect| book_row(None, format!("● {effect}"), None, None)));
    }
    Some(paginate("tconstruct:modifier", Some(modifier_name(modifier)), text, rows, Vec::new()))
}

/* The tag injectors (ToolTagInjector / ModifierTagInjector over AbstractTagInjectingTransformer) */

/// One injection spec: a tag whose members each get a page, the folder their data files live in relative
/// to the section's own folder, whether the pages sort by title, and an optional page-name prefix.
struct LoadSpec {
    tag: Identifier,
    path: String,
    sort: bool,
    prefix: Option<String>,
}

fn load_specs(extra: &serde_json::Map<String, Value>, key: &str) -> Vec<LoadSpec> {
    let Some(value) = extra.get(key) else { return Vec::new() };
    let values: Vec<&Value> = match value { Value::Array(list) => list.iter().collect(), single => vec![single] };
    values.iter().filter_map(|spec| {
        let tag = Identifier::parse(spec.get("tag")?.as_str()?).ok()?;
        Some(LoadSpec {
            tag,
            path: spec.get("path").and_then(Value::as_str).unwrap_or_default().to_owned(),
            sort: spec.get("sort").and_then(Value::as_bool).unwrap_or(true),
            prefix: spec.get("prefix").and_then(Value::as_str).map(str::to_owned),
        })
    }).collect()
}

/// The data file of a generated page under a transformer's folder, when the book has one for it (the
/// page language's folder, else the English one — the same fallback the section files load by). The
/// folder is the spec's own path under the language folder, the layout the shipped books use
/// (`en_us/tools/tconstruct_pickaxe.json`, `en_us/fluids/tconstruct_water.json`).
fn generated_json(read: &Read<'_>, book: &str, language: &str, folder: &str, id: &Identifier) -> Option<Value> {
    let file = if folder.is_empty() { format!("{}_{}.json", id.namespace(), id.path()) } else { format!("{folder}/{}_{}.json", id.namespace(), id.path()) };
    read_json(read, &format!("book/{book}/{language}/{file}")).or_else(|| read_json(read, &format!("book/{book}/en_us/{file}")))
}

/// Injects the pages one spec names into `pages` at `at`: one per tag member, a data file's content where
/// the folder has one and the fallback content otherwise, sorted by title when the spec says so.
fn inject(read: &Read<'_>, book: &str, language: &str, key: &str, spec: &LoadSpec, data: &GenData, pages: &mut Vec<RawPage>, at: usize) -> usize {
    let mut injected: Vec<RawPage> = Vec::new();
    if key == "tconstruct:load_tools" {
        let Some(items) = &data.items else { return 0 };
        let Some(members) = items.get_tag(&spec.tag) else { return 0 };
        for item in members {
            let json = generated_json(read, book, language, &spec.path, item);
            let tool = json.as_ref().and_then(|json| json.get("tool")).and_then(Value::as_str).and_then(|tool| Identifier::parse(tool).ok()).unwrap_or_else(|| item.clone());
            injected.push(RawPage {
                name: Some(page_name(spec, item)),
                kind: Some("tconstruct:tool".to_owned()),
                data: None,
                extra: serde_json::Map::new(),
                pages: tool_pages(&tool, json.as_ref()),
                content: Some(GenPage::Tool(tool)),
            });
        }
    } else {
        let Some(modifiers) = &data.modifiers else { return 0 };
        let Some(members) = modifiers.tags.get(&spec.tag) else { return 0 };
        for id in members {
            let json = generated_json(read, book, language, &spec.path, id);
            let modifier = json.as_ref().and_then(|json| json.get("modifier_id")).and_then(Value::as_str).and_then(|modifier| Identifier::parse(modifier).ok()).unwrap_or_else(|| id.clone());
            let Some(pages) = modifier_pages(modifiers, &modifier, json.as_ref()) else { continue };
            injected.push(RawPage {
                name: Some(page_name(spec, id)),
                kind: Some("tconstruct:modifier".to_owned()),
                data: None,
                extra: serde_json::Map::new(),
                pages,
                content: Some(GenPage::Modifier(modifier)),
            });
        }
    }
    if spec.sort {
        injected.sort_by(|a, b| a.title().unwrap_or_default().cmp(b.title().unwrap_or_default()));
    }
    let count = injected.len();
    for (offset, page) in injected.into_iter().enumerate() { pages.insert(at + offset, page); }
    count
}

/// An injected page's name: `[prefix.]<namespace>.<path>`, the dotted form the original names them by.
fn page_name(spec: &LoadSpec, id: &Identifier) -> String {
    let base = format!("{}.{}", id.namespace(), id.path());
    match &spec.prefix { Some(prefix) => format!("{prefix}.{base}"), None => base }
}

/// Runs the injectors over one section: the section's own extraData first (at the head), then each
/// page's, the pages it names landing right after the page that named them (the transformer's order).
fn inject_section(read: &Read<'_>, book: &str, language: &str, section: &mut RawSection, data: &GenData) {
    let mut at = 0;
    for key in ["tconstruct:load_tools", "tconstruct:load_modifiers"] {
        for spec in load_specs(&section.extra.clone(), key) {
            at += inject(read, book, language, key, &spec, data, &mut section.pages, at);
        }
    }
    let mut index = at;
    while index < section.pages.len() {
        let extra = section.pages[index].extra.clone();
        let mut added = 0;
        for key in ["tconstruct:load_tools", "tconstruct:load_modifiers"] {
            for spec in load_specs(&extra, key) {
                added += inject(read, book, language, key, &spec, data, &mut section.pages, index + 1 + added);
            }
        }
        index += 1 + added;
    }
}

/* Material pages and the tier range transformer (TierRangeMaterialSectionTransformer) */

/// A `tconstruct:` id from its path (the stat types and material types all live there).
fn tid(path: &str) -> Identifier { Identifier::new("tconstruct", path).expect("a static tconstruct id") }

/// One material content type of TinkerBook's registerMaterialType: its page kind, the stat types a
/// material must have one of to be listed, the stat blocks its pages show in index order, the flavor
/// text suffix, and how its materials sort.
struct MaterialType {
    kind: &'static str,
    visible: &'static [&'static str],
    blocks: &'static [&'static str],
    suffix: &'static str,
    sort: fn(&Definitions, &Identifier, &Identifier) -> std::cmp::Ordering,
}

fn sort_none(_: &Definitions, _: &Identifier, _: &Identifier) -> std::cmp::Ordering { std::cmp::Ordering::Equal }

fn has_stat(definitions: &Definitions, material: &Identifier, stat: &str) -> bool {
    definitions.get_stats(material, &tid(stat)).is_some()
}

/// Melee/harvest: materials with a head first.
fn sort_melee(definitions: &Definitions, a: &Identifier, b: &Identifier) -> std::cmp::Ordering {
    has_stat(definitions, b, "head").cmp(&has_stat(definitions, a, "head"))
}

/// Ranged: bowstring materials first, then limb materials last.
fn sort_ranged(definitions: &Definitions, a: &Identifier, b: &Identifier) -> std::cmp::Ordering {
    has_stat(definitions, b, "bowstring").cmp(&has_stat(definitions, a, "bowstring"))
        .then(has_stat(definitions, a, "limb").cmp(&has_stat(definitions, b, "limb")))
}

/// Ammo: arrow head materials first, then fletching materials last, then arrow shaft materials last.
fn sort_ammo(definitions: &Definitions, a: &Identifier, b: &Identifier) -> std::cmp::Ordering {
    has_stat(definitions, b, "arrow_head").cmp(&has_stat(definitions, a, "arrow_head"))
        .then(has_stat(definitions, a, "fletching").cmp(&has_stat(definitions, b, "fletching")))
        .then(has_stat(definitions, a, "arrow_shaft").cmp(&has_stat(definitions, b, "arrow_shaft")))
}

/// Armor's bucket: cuirass-only materials, then cuirass with maille, then maille-only, then anything
/// with chestplate plating, then the rest (shield cores).
fn armor_bucket(definitions: &Definitions, material: &Identifier) -> u8 {
    let cuirass = has_stat(definitions, material, "cuirass");
    let maille = has_stat(definitions, material, "maille");
    if cuirass && !maille { 1 }
    else if cuirass { 2 }
    else if maille { 3 }
    else if has_stat(definitions, material, "plating_chestplate") { 4 }
    else { 5 }
}

fn sort_armor(definitions: &Definitions, a: &Identifier, b: &Identifier) -> std::cmp::Ordering {
    armor_bucket(definitions, a).cmp(&armor_bucket(definitions, b))
}

/// Skull: the order of the `tconstruct:slimeskull` material tag, materials outside it last.
fn sort_skull(definitions: &Definitions, a: &Identifier, b: &Identifier) -> std::cmp::Ordering {
    let order = definitions.tags.get(&tid("slimeskull"));
    let position = |material: &Identifier| order.and_then(|members| members.iter().position(|member| member == material)).unwrap_or(usize::MAX);
    position(a).cmp(&position(b))
}

const MATERIAL_TYPES: &[(&str, MaterialType)] = &[
    ("tconstruct:melee_harvest", MaterialType { kind: "tconstruct:melee_harvest_material", visible: &["head", "handle", "binding"], blocks: &["head", "handle", "binding"], suffix: "melee_harvest", sort: sort_melee }),
    ("tconstruct:ranged", MaterialType { kind: "tconstruct:ranged_material", visible: &["limb", "grip", "bowstring"], blocks: &["limb", "grip", "bowstring"], suffix: "ranged", sort: sort_ranged }),
    ("tconstruct:ammo", MaterialType { kind: "tconstruct:ammo_material", visible: &["arrow_head", "arrow_shaft", "fletching"], blocks: &["arrow_head", "arrow_shaft", "fletching"], suffix: "ammo", sort: sort_ammo }),
    ("tconstruct:armor", MaterialType { kind: "tconstruct:armor_material", visible: &["plating_helmet", "plating_chestplate", "plating_leggings", "plating_boots", "plating_shield", "maille", "cuirass", "shield_core"], blocks: &[], suffix: "armor", sort: sort_armor }),
    ("tconstruct:slime", MaterialType { kind: "tconstruct:slime_material", visible: &["slime"], blocks: &["slime"], suffix: "slime", sort: sort_none }),
    ("tconstruct:skull", MaterialType { kind: "tconstruct:skull_material", visible: &["skull"], blocks: &["skull"], suffix: "skull", sort: sort_skull }),
    ("tconstruct:ribcage", MaterialType { kind: "tconstruct:ribcage_material", visible: &["ribcage"], blocks: &["ribcage"], suffix: "ribcage", sort: sort_none }),
    ("tconstruct:shell", MaterialType { kind: "tconstruct:shell_material", visible: &["shell"], blocks: &["shell"], suffix: "shell", sort: sort_none }),
    ("tconstruct:laces", MaterialType { kind: "tconstruct:laces_material", visible: &["laces"], blocks: &["laces"], suffix: "laces", sort: sort_none }),
];

fn material_type(id: &str) -> Option<&'static MaterialType> {
    MATERIAL_TYPES.iter().find(|(name, _)| *name == id).map(|(_, kind)| kind)
}

/// A material predicate (MaterialPredicate), in the forms the shipped book data uses: a material tag,
/// an id, or/and, an inversion, and a load condition choosing between two predicates. Anything else
/// matches nothing and is logged, rather than guessing at its meaning.
fn predicate_matches(predicate: &Value, material: &Identifier, definitions: &Definitions, data: &GenData) -> bool {
    let Some(kind) = predicate.get("type").and_then(Value::as_str) else { return false };
    match kind {
        "mantle:tag" => predicate.get("tag").and_then(Value::as_str).and_then(|tag| Identifier::parse(tag).ok())
            .is_some_and(|tag| definitions.tags.get(&tag).is_some_and(|members| members.contains(material))),
        "tconstruct:id" => predicate.get("id").and_then(Value::as_str).and_then(|id| Identifier::parse(id).ok()).is_some_and(|id| id == *material),
        "mantle:or" => predicate.get("predicates").and_then(Value::as_array)
            .is_some_and(|list| list.iter().any(|inner| predicate_matches(inner, material, definitions, data))),
        "mantle:and" => predicate.get("predicates").and_then(Value::as_array)
            .is_some_and(|list| list.iter().all(|inner| predicate_matches(inner, material, definitions, data))),
        "mantle:inverted" => {
            // The wrapped predicate's fields sit in the same object; its type is named by inverted_type.
            let Some(inner_type) = predicate.get("inverted_type").and_then(Value::as_str) else { return false };
            let mut inner = predicate.as_object().cloned().unwrap_or_default();
            inner.remove("inverted_type");
            inner.insert("type".to_owned(), Value::String(inner_type.to_owned()));
            !predicate_matches(&Value::Object(inner), material, definitions, data)
        }
        "mantle:load_condition" => {
            let conditions = predicate.get("conditions").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
            let met = conditions.iter().all(|condition| book_condition_met(condition, data));
            let branch = if met { predicate.get("if_true") } else { predicate.get("if_false") };
            branch.is_some_and(|branch| predicate_matches(branch, material, definitions, data))
        }
        other => {
            eprintln!("Tinkers: book material predicate of unknown type {other}, matching nothing");
            false
        }
    }
}

/// One condition of a load condition predicate, in the form the shipped data uses: a tag filled in the
/// item registry. Without a captured item registry the tag counts as unfilled.
fn book_condition_met(condition: &Value, data: &GenData) -> bool {
    match condition.get("type").and_then(Value::as_str) {
        Some("mantle:tag_filled") => {
            let Some(tag) = condition.get("tag").and_then(Value::as_str).and_then(|tag| Identifier::parse(tag).ok()) else { return false };
            data.items.as_ref().is_some_and(|items| items.get_tag(&tag).is_some_and(|members| !members.is_empty()))
        }
        other => {
            eprintln!("Tinkers: book load condition of unknown type {other:?}, treated as unmet");
            false
        }
    }
}

/// A section's own condition, in the one form the shipped books use: shown when a registry tag is not
/// empty. Without a captured item registry the section is kept (its pages are the book's own armor
/// pages); an unknown condition type is likewise kept and logged rather than hiding a section.
fn section_condition_met(condition: &Value, data: &GenData) -> bool {
    match condition.get("type").and_then(Value::as_str) {
        Some("tconstruct:tag_not_empty") => {
            let (Some(registry), Some(tag)) = (condition.get("registry").and_then(Value::as_str), condition.get("tag").and_then(Value::as_str).and_then(|tag| Identifier::parse(tag).ok())) else { return true };
            if registry != "minecraft:item" { return true; }
            match &data.items {
                Some(items) => items.get_tag(&tag).is_some_and(|members| !members.is_empty()),
                None => true,
            }
        }
        other => {
            eprintln!("Tinkers: book section condition of unknown type {other:?}, keeping the section");
            true
        }
    }
}

/// One tier spec of a material section: a material type, the tier range (a bare number is that tier
/// exactly, a range object its bounds), the tag or predicate choosing materials, whether the pages are
/// the encyclopedia's detailed ones, the page name root, and a title suffix translation key.
struct TierSpec {
    kind: &'static MaterialType,
    min: i32,
    max: i32,
    tag: Option<Identifier>,
    predicate: Option<Value>,
    detailed: bool,
    name: String,
    suffix: Option<String>,
}

fn tier_specs(extra: &serde_json::Map<String, Value>) -> Vec<TierSpec> {
    let Some(value) = extra.get("tconstruct:material_tier") else { return Vec::new() };
    let values: Vec<&Value> = match value { Value::Array(list) => list.iter().collect(), single => vec![single] };
    values.iter().filter_map(|spec| {
        let type_id = spec.get("type")?.as_str()?;
        let kind = material_type(type_id)?;
        let (min, max) = match spec.get("tier") {
            Some(Value::Number(number)) => { let tier = number.as_i64()? as i32; (tier, tier) }
            Some(range) => (
                range.get("min").and_then(Value::as_i64).unwrap_or(0) as i32,
                range.get("max").and_then(Value::as_i64).unwrap_or(32767) as i32,
            ),
            None => (0, 32767),
        };
        Some(TierSpec {
            kind,
            min,
            max,
            tag: spec.get("tag").and_then(Value::as_str).and_then(|tag| Identifier::parse(tag).ok()),
            predicate: spec.get("predicate").cloned(),
            detailed: spec.get("detailed").and_then(Value::as_bool).unwrap_or(false),
            name: spec.get("name").and_then(Value::as_str).map(str::to_owned)
                .unwrap_or_else(|| type_id.split(':').nth(1).unwrap_or(type_id).to_owned()),
            suffix: spec.get("suffix").and_then(Value::as_str).map(str::to_owned),
        })
    }).collect()
}

/// The materials one tier spec lists, in the book's order: the registry's visible materials in their
/// order, filtered to the tier range, the tag or predicate, and having one of the type's stats, then
/// stably sorted by the type's comparator (ValidMaterial and the transformer's sort).
fn select_materials(definitions: &Definitions, spec: &TierSpec, data: &GenData) -> Vec<Identifier> {
    let mut selected: Vec<Identifier> = definitions.visible.iter().filter(|id| {
        let Some(material) = definitions.materials.get(*id) else { return false };
        if material.tier < spec.min || material.tier > spec.max { return false; }
        if let Some(tag) = &spec.tag {
            if !definitions.tags.get(tag).is_some_and(|members| members.contains(*id)) { return false; }
        }
        if let Some(predicate) = &spec.predicate {
            if !predicate_matches(predicate, id, definitions, data) { return false; }
        }
        spec.kind.visible.iter().any(|stat| has_stat(definitions, id, stat))
    }).cloned().collect();
    selected.sort_by(|a, b| (spec.kind.sort)(definitions, a, b));
    selected
}

/// A material's trait rows for one stat type: one row per bound trait whose modifier shows in books,
/// by its display name (getTraits with shouldDisplay(ModifierTooltip.BOOK)).
fn trait_rows(definitions: &Definitions, data: &GenData, material: &Identifier, stat: &str) -> Vec<BookRow> {
    let Some(modifiers) = &data.modifiers else { return Vec::new() };
    definitions.get_traits(material, &tid(stat)).iter().flatten().filter_map(|entry| {
        let modifier = modifiers.get(&entry.id)?;
        if !modifier.tooltips.contains("book") { return None; }
        Some(book_row(None, modifier_name(modifier), None, None))
    }).collect()
}

/// A material page's title: its display name, with the suffix component appended through the tool
/// format string when the content or the tier spec sets one.
fn material_title(material: &Identifier, suffix: Option<&str>) -> String {
    let name = commands::material_id_name(material);
    match suffix {
        Some(key) => tr_args("item.tconstruct.tool.format", &[&name, &tr(key)]),
        None => name,
    }
}

/// A material's flavor text (getTextKey): the detailed key in the encyclopedia, the flavor key
/// elsewhere, a type-specific suffix key winning when it has a translation, and the earlier
/// type-named keys a few contents check first. Flavor text is quoted; nothing when no key translates.
fn flavor_text(kind: &MaterialType, material: &Identifier, detailed: bool) -> Option<String> {
    let root = format!("material.{}.{}", material.namespace(), material.path());
    // The earlier keys: skull checks its own always; ranged, ammo and armor check theirs when detailed.
    let earlier = if kind.suffix == "skull" {
        Some(format!("{root}.{}", if detailed { "skull_encyclopedia" } else { "skull_flavor" }))
    } else if detailed && ["ranged", "ammo", "armor"].contains(&kind.suffix) {
        Some(format!("{root}.{}", kind.suffix))
    } else { None };
    if let Some(earlier) = &earlier {
        if I18n::has(earlier) { return Some(flavor(&tr(earlier), detailed)); }
    }
    let base = format!("{root}.{}", if detailed { "encyclopedia" } else { "flavor" });
    let suffixed = format!("{base}.{}", kind.suffix);
    let text = if I18n::has(&suffixed) { tr(&suffixed) } else if I18n::has(&base) { tr(&base) } else { return None };
    Some(flavor(&text, detailed))
}

fn flavor(text: &str, detailed: bool) -> String {
    if detailed { text.to_owned() } else { format!("\"{text}\"") }
}

/// Builds one material's pages for a content type: a stat block per stat type the material has (the
/// type's localized name, its stat lines, its trait lines), then the flavor text. The armor type builds
/// its combined plating block instead (see armor_material_pages).
fn material_pages(definitions: &Definitions, data: &GenData, kind: &MaterialType, material: &Identifier, detailed: bool, title: Option<String>, suffix: Option<&str>) -> Vec<BookPage> {
    let title = title.unwrap_or_else(|| material_title(material, suffix));
    if kind.suffix == "armor" {
        return armor_material_pages(definitions, data, material, detailed, title);
    }
    let mut rows = Vec::new();
    for stat in kind.blocks {
        let stat_id = tid(stat);
        let Some(stats) = definitions.get_stats(material, &stat_id) else { continue };
        rows.push(BookRow::Header { icon: None, text: commands::stat_type_name(&stat_id) });
        rows.extend(commands::localized_info(stats, &stat_id, 1.0).into_iter().map(|line| book_row(None, line, None, None)));
        rows.extend(trait_rows(definitions, data, material, stat));
    }
    let text = flavor_text(kind, material, detailed).map(|text| vec![pluto_lib::BookText::plain(&text)]).unwrap_or_default();
    with_sidebar(paginate(kind.kind, Some(title), Vec::new(), rows, text), material_part_items(material))
}

/// The sidebar items of a material's pages: every part the part builder makes, in this material
/// (a part stack names its material in its own data, the form the part models select on).
fn material_part_items(material: &Identifier) -> Vec<pluto_lib::ItemSlot> {
    crate::tool_parts::material_items().into_keys().map(|part| {
        let mut stack = pluto_lib::ItemSlot::new(part, 1);
        stack.custom_data.insert("Material".to_owned(), Nbt::String(material.to_string()));
        stack
    }).collect()
}

/// The armor material page: the combined plating block (one line per property, the present plating
/// stats' values joined top-down, or the single value when all agree), the helmet and shield trait
/// blocks, then a traits-only block per remaining stat type, then the flavor text.
fn armor_material_pages(definitions: &Definitions, data: &GenData, material: &Identifier, detailed: bool, title: String) -> Vec<BookPage> {
    let mut rows = Vec::new();
    let plating: Vec<&Stats> = ["plating_helmet", "plating_chestplate", "plating_leggings", "plating_boots", "plating_shield"]
        .iter().filter_map(|stat| definitions.get_stats(material, &tid(stat))).collect();
    if !plating.is_empty() {
        rows.push(BookRow::Header { icon: None, text: tr("stat.tconstruct.plating") });
        let lines: [(&str, fn(&Stats) -> String); 4] = [
            ("durability", |stats| match stats { Stats::Plating { durability, .. } => commands::comma_int(*durability), _ => String::new() }),
            ("armor", |stats| match stats { Stats::Plating { armor, .. } => commands::comma(f64::from(*armor)), _ => String::new() }),
            ("armor_toughness", |stats| match stats { Stats::Plating { toughness, .. } => commands::comma(f64::from(*toughness)), _ => String::new() }),
            ("knockback_resistance", |stats| match stats { Stats::Plating { knockback_resistance, .. } => commands::comma(f64::from(*knockback_resistance) * 10.0), _ => String::new() }),
        ];
        for (property, value) in lines {
            let values: Vec<String> = plating.iter().map(|stats| value(stats)).collect();
            let joined = if values.iter().all(|other| other == &values[0]) { values[0].clone() } else { values.join(" / ") };
            rows.push(book_row(None, tr(&format!("tool_stat.tconstruct.{property}")), Some(joined), None));
        }
    }
    for (stat, header) in [("plating_helmet", "stat.tconstruct.plating_armor"), ("plating_shield", "stat.tconstruct.plating_shield")] {
        if definitions.get_stats(material, &tid(stat)).is_some() {
            rows.push(BookRow::Header { icon: None, text: tr(header) });
            rows.extend(trait_rows(definitions, data, material, stat));
        }
    }
    for stat in ["maille", "cuirass", "shield_core"] {
        if definitions.get_stats(material, &tid(stat)).is_some() {
            rows.push(BookRow::Header { icon: None, text: commands::stat_type_name(&tid(stat)) });
            rows.extend(trait_rows(definitions, data, material, stat));
        }
    }
    let text = flavor_text(material_type("tconstruct:armor").expect("the armor type"), material, detailed)
        .map(|text| vec![pluto_lib::BookText::plain(&text)]).unwrap_or_default();
    with_sidebar(paginate("tconstruct:armor_material", Some(title), Vec::new(), rows, text), material_part_items(material))
}

/// The overview pages of a generated section: one entry per content page group, icon and title, linking
/// to the group's first page. Every overview page carries the section's title and subtext, as the
/// original's icon lists do. `groups` pairs each entry's icon and title with how many pages its content
/// makes, in order.
fn overview_pages(title: &str, subtext: Option<&str>, groups: &[(Option<BookIcon>, String, usize)]) -> Vec<BookPage> {
    let intro = subtext.map(|subtext| vec![pluto_lib::BookText::plain(subtext)]).unwrap_or_default();
    let mut target = 0; // shifted past the overview pages themselves once their count is known
    let rows: Vec<BookRow> = groups.iter().map(|(icon, label, pages)| {
        let row = book_row(icon.clone(), label.clone(), None, Some(target));
        target += pages;
        row
    }).collect();
    let mut pages = paginate("mantle:index", Some(title.to_owned()), intro, rows, Vec::new());
    let count = pages.len();
    shift_links(&mut pages, count);
    pages
}

/// A material's overview icon: a repair kit carrying the material, the display stack the original
/// itself falls back to when a material has no better one.
fn material_icon(material: &Identifier) -> BookIcon {
    BookIcon::ItemWithData {
        item: tid("repair_kit"),
        data: [("Material".to_owned(), Nbt::String(material.to_string()))].into_iter().collect(),
    }
}

/// Runs the tier range transformer over one section: for each tier spec, an overview of its materials
/// and a page per material go in at the head, before the section's own pages. A spec naming materials
/// already listed by an earlier spec of the section lists them again under its own page names, as the
/// original's per-spec pass does; here the pages are the same content either way.
fn tier_transform(section: &mut RawSection, table: &[(String, String)], data: &GenData) {
    let specs = tier_specs(&section.extra);
    if specs.is_empty() { return; }
    let Some(definitions) = &data.materials else { return };
    let subtext = lookup(table, &format!("{}.subtext", section.name));
    let mut generated: Vec<BookPage> = Vec::new();
    for spec in &specs {
        let materials = select_materials(definitions, spec, data);
        if materials.is_empty() { continue; }
        let mut groups: Vec<(Option<BookIcon>, String, usize)> = Vec::new();
        let mut pages: Vec<BookPage> = Vec::new();
        for material in &materials {
            let built = material_pages(definitions, data, spec.kind, material, spec.detailed, None, spec.suffix.as_deref());
            groups.push((Some(material_icon(material)), material_title(material, spec.suffix.as_deref()), built.len()));
            pages.extend(built);
        }
        generated.extend(overview_pages(&section.title, subtext, &groups));
        generated.extend(pages);
    }
    if generated.is_empty() { return; }
    let mut pages: Vec<RawPage> = generated.into_iter().map(|page| RawPage { name: None, kind: None, data: None, extra: serde_json::Map::new(), pages: vec![page], content: None }).collect();
    pages.append(&mut section.pages);
    section.pages = pages;
}

/* Fluid effect pages (FluidEffectInjectingTransformer and the effects' own descriptions) */

/// Joins description parts with the separator translation, pairwise (MERGE_COMPONENT_LIST).
fn merge_descriptions(parts: Vec<String>) -> String {
    parts.into_iter().reduce(|left, right| tr_args("fluid_effect.tconstruct.separator", &[&left, &right])).unwrap_or_default()
}

/// An effect's plain description: its loader's translation key, no arguments (the default getDescription).
fn plain_description(loader: &str) -> String { tr(&format!("fluid_effect.tconstruct.{loader}")) }

/// A mob effect's display name at a time action (MobEffectInstance's book name): the effect's name,
/// amplified through the potion format when above level 1, and — unless the time is infinite — the
/// action's per-level seconds format around it.
fn mob_effect_name(effect: &MobEffectDef, action: TimeAction) -> String {
    let base = tr(&format!("effect.{}.{}", effect.effect.namespace(), effect.effect.path()));
    let base = if effect.level > 1 {
        tr_args("potion.withAmplifier", &[&base, &tr(&format!("potion.potency.{}", effect.level - 1))])
    } else { base };
    if effect.time < 0 { return base; }
    let key = match action { TimeAction::Add => "fluid_effect.tconstruct.mob_effect.add", TimeAction::Set => "fluid_effect.tconstruct.mob_effect.set" };
    tr_args(key, &[&(effect.time / 20).to_string(), &base])
}

fn action_name(action: TimeAction) -> &'static str {
    match action { TimeAction::Add => "add", TimeAction::Set => "set" }
}

/// An entity effect's book description line, following its own getDescription.
fn entity_description(effect: &EntityEffect) -> String {
    match effect {
        EntityEffect::Conditional { effect, .. } => entity_description(effect),
        EntityEffect::Scaling { effects } => effects.last().map(|(_, inner)| entity_description(inner)).unwrap_or_default(),
        EntityEffect::Alternatives { effects } => effects.first().map(entity_description).unwrap_or_default(),
        EntityEffect::Sequence { effects } => merge_descriptions(effects.iter().map(entity_description).collect()),
        EntityEffect::Calcified => plain_description("calcified"),
        EntityEffect::Extinguish => plain_description("extinguish"),
        EntityEffect::Teleport { .. } => plain_description("teleport"),
        EntityEffect::CureEffects { .. } => plain_description("cure_effects"),
        EntityEffect::RemoveEffect { effect } => tr_args("fluid_effect.tconstruct.remove_effect", &[&tr(&format!("effect.{}.{}", effect.namespace(), effect.path()))]),
        EntityEffect::MobEffect { def, action } => mob_effect_name(def, *action),
        EntityEffect::Potion { .. } => plain_description("potion"),
        // The original appends the damage type's message id; no registry this port keeps carries it, so
        // the line uses the unsuffixed key (see the module note).
        EntityEffect::Damage { damage, .. } => tr_args("fluid_effect.tconstruct.damage", &[&float_text(*damage)]),
        EntityEffect::RestoreHunger { hunger, .. } => tr_args("fluid_effect.tconstruct.restore_hunger", &[&hunger.to_string()]),
        EntityEffect::Fire { action, time } => tr_args(&format!("fluid_effect.tconstruct.fire.{}", action_name(*action)), &[&time.to_string()]),
        EntityEffect::Freeze { action, time } => tr_args(&format!("fluid_effect.tconstruct.freeze.{}", action_name(*action)), &[&(*time / 20).to_string()]),
        EntityEffect::AwardStat { stat, amount } => {
            let key = format!("fluid_effect.tconstruct.award_stat.{}", if *amount >= 0 { "add" } else { "subtract" });
            tr_args(&key, &[&commands::comma_int(amount.abs()), &tr(&format!("stat.{}.{}", stat.namespace(), stat.path()))])
        }
        EntityEffect::AddBreath { amount } => {
            let key = format!("fluid_effect.tconstruct.add_breath.{}", if *amount >= 0 { "add" } else { "subtract" });
            tr_args(&key, &[&commands::comma(f64::from(amount.unsigned_abs()) / 30.0)])
        }
        EntityEffect::PushEntity { value } => plain_description(if value.compute(1.0) >= 0.0 { "push_entity.push" } else { "push_entity.pull" }),
        EntityEffect::Interact => tr("fluid_effect.tconstruct.interact.entity"),
        EntityEffect::General(general) => general_description(general),
    }
}

/// A block effect's book description line, following its own getDescription.
fn block_description(effect: &BlockEffect) -> String {
    match effect {
        BlockEffect::Conditional { effect, .. } => block_description(effect),
        BlockEffect::Scaling { effects } => effects.last().map(|(_, inner)| block_description(inner)).unwrap_or_default(),
        BlockEffect::Alternatives { effects } => effects.first().map(block_description).unwrap_or_default(),
        BlockEffect::Sequence { effects } => merge_descriptions(effects.iter().map(block_description).collect()),
        BlockEffect::Offset { effect, .. } => block_description(effect),
        BlockEffect::Weather => plain_description("weather"),
        BlockEffect::RemoveBlock => plain_description("remove_block"),
        BlockEffect::PlaceBlock { block, .. } => match block {
            None => tr("fluid_effect.tconstruct.place_block.held"),
            Some(block) => tr_args("fluid_effect.tconstruct.place_block", &[&tr(&format!("block.{}.{}", block.block.namespace(), block.block.path()))]),
        },
        BlockEffect::BreakBlock { hardness, enchantments } => {
            // The original names each enchantment with its level when the enchantment's max level is
            // not 1; the port keeps no max levels, so the level is named when above 1 — the same on
            // the shipped data, whose one enchantment (fortune) both readings name.
            let names = merge_descriptions(enchantments.iter().map(|(id, level)| {
                let name = tr(&format!("enchantment.{}.{}", id.namespace(), id.path()));
                if *level > 1 { format!("{} {}", name, tr(&format!("enchantment.level.{level}"))) } else { name }
            }).collect());
            let hardness = *hardness;
            if enchantments.is_empty() {
                if hardness == 0.0 { plain_description("break_block") }
                else { tr_args("fluid_effect.tconstruct.break_block.hardness", &[&float_text(hardness)]) }
            } else if hardness == 0.0 {
                tr_args("fluid_effect.tconstruct.break_block.enchanted", &[&names])
            } else {
                tr_args("fluid_effect.tconstruct.break_block.enchanted.hardness", &[&float_text(hardness), &names])
            }
        }
        BlockEffect::MobEffectCloud { effects } => {
            let names = merge_descriptions(effects.iter().map(|effect| mob_effect_name(effect, TimeAction::Set)).collect());
            tr_args("fluid_effect.tconstruct.mob_effect_cloud", &[&names])
        }
        BlockEffect::PotionCloud { .. } => plain_description("potion_cloud"),
        BlockEffect::MoveBlock { push, .. } => plain_description(if *push { "move_block.push" } else { "move_block.pull" }),
        BlockEffect::Interact => tr("fluid_effect.tconstruct.interact.block"),
        BlockEffect::MeltBlock { .. } => plain_description("melt_block"),
        BlockEffect::General(general) => general_description(general),
    }
}

/// A general effect's book description line, following its own getDescription.
fn general_description(effect: &GeneralEffect) -> String {
    match effect {
        GeneralEffect::DropItem { item } => {
            let name = match &item.source { Source::Id(id) => item_name(id), Source::Tag(tag) => tag.to_string() };
            tr_args("fluid_effect.tconstruct.drop_item", &[&name])
        }
        GeneralEffect::Explosion { radius, .. } => tr_args("fluid_effect.tconstruct.explosion", &[&float_text(radius.compute(1.0))]),
        GeneralEffect::SetBlock { block } => tr_args("fluid_effect.tconstruct.set_block", &[&tr(&format!("block.{}.{}", block.block.namespace(), block.block.path()))]),
        GeneralEffect::AreaMobEffect { def, action, .. } => mob_effect_name(def, *action),
    }
}

/// The fluids an entry's ingredient names: fluid ids directly, tags through the captured fluid
/// registry, in option order without repeats (the transformer's check that an entry has fluids).
fn ingredient_fluids(effects: &FluidEffects, data: &GenData) -> Vec<Identifier> {
    let mut fluids: Vec<Identifier> = Vec::new();
    for (source, _) in &effects.ingredient().options {
        match source {
            Source::Id(fluid) => { if !fluids.contains(fluid) { fluids.push(fluid.clone()); } }
            Source::Tag(tag) => {
                if let Some(members) = data.fluids.as_ref().and_then(|snapshot| snapshot.get_tag(tag)) {
                    for fluid in members { if !fluids.contains(fluid) { fluids.push(fluid.clone()); } }
                }
            }
        }
    }
    fluids
}

/// Builds one fluid entry's pages: the fluid's name as title, its description text, then the entity
/// effect list and the block effect list under their headers, each bulleted. A data file's own title,
/// text and lists win where it has them; the lists otherwise generate from the entry's effects
/// (loadEffectData). The overview icon is the first fluid's bucket item, where that item exists.
fn fluid_page(read: &Read<'_>, book: &str, language: &str, folder: &str, id: &Identifier, effects: &FluidEffects, fluids: &[Identifier], data: &GenData) -> (String, Option<BookIcon>, Vec<BookPage>) {
    let json = generated_json(read, book, language, folder, id);
    let title = json.as_ref().and_then(|json| json.get("title")).and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty()).map(str::to_owned)
        .unwrap_or_else(|| tr(&format!("fluid.{}.{}", id.namespace(), id.path())));
    let mut text = json.as_ref().map(|json| text_runs(json.get("text"))).unwrap_or_default();
    if text.iter().all(|run| run.text.trim().is_empty()) {
        text = vec![pluto_lib::BookText::plain(&tr(&format!("fluid.{}.{}.fluid_effect", id.namespace(), id.path())))];
    }
    let list = |key: &str, generated: Option<Vec<String>>| -> Option<Vec<String>> {
        match json.as_ref().and_then(|json| json.get(key)) {
            Some(value) => Some(string_list(Some(value))),
            None => generated,
        }
    };
    let entity = list("entity", effects.has_entity_effects().then(|| effects.entity_effects().iter().map(entity_description).collect()));
    let block = list("block", effects.has_block_effects().then(|| effects.block_effects().iter().map(block_description).collect()));
    let mut rows = Vec::new();
    for (header, lines) in [("book.tconstruct.fluid_effects.entity", entity), ("book.tconstruct.fluid_effects.block", block)] {
        if let Some(lines) = lines {
            rows.push(BookRow::Header { icon: None, text: tr(header) });
            rows.extend(lines.into_iter().map(|line| book_row(None, format!("● {line}"), None, None)));
        }
    }
    let icon = fluids.first().and_then(|fluid| {
        let bucket = Identifier::new(fluid.namespace(), &format!("{}_bucket", fluid.path())).ok()?;
        data.items.as_ref().is_some_and(|items| items.contains(&bucket)).then_some(BookIcon::Item(bucket))
    });
    // The sidebar: one bucket per fluid the entry names, where that bucket item exists.
    let buckets: Vec<pluto_lib::ItemSlot> = fluids.iter().filter_map(|fluid| {
        let bucket = Identifier::new(fluid.namespace(), &format!("{}_bucket", fluid.path())).ok()?;
        data.items.as_ref().is_some_and(|items| items.contains(&bucket)).then(|| pluto_lib::ItemSlot::new(bucket, 1))
    }).collect();
    (title.clone(), icon, with_sidebar(paginate("tconstruct:fluid_effect", Some(title), text, rows, Vec::new()), buckets))
}

/// Runs the fluid effect transformer over the encyclopedia's fluid section: an overview of the
/// entries and a page per entry go in at the head — the spec's order list first (its bare names are
/// tconstruct's), the rest by title. An entry whose ingredient names no fluids ends the generation,
/// the original's own early return; hidden entries are skipped.
fn fluid_transform(read: &Read<'_>, book: &str, language: &str, section: &mut RawSection, table: &[(String, String)], data: &GenData) {
    let Some(spec) = section.extra.get("tconstruct:fluid_effects") else { return };
    let Some(entries) = &data.fluid_effects else { return };
    let folder = spec.get("path").and_then(Value::as_str).unwrap_or_default().to_owned();
    let order = string_list(spec.get("order"));
    let mut built: Vec<(Identifier, String, Option<BookIcon>, Vec<BookPage>)> = Vec::new();
    for (id, effects) in entries {
        if effects.hidden { continue; }
        let fluids = ingredient_fluids(effects, data);
        if fluids.is_empty() { return; }
        let (title, icon, pages) = fluid_page(read, book, language, &folder, id, effects, &fluids, data);
        built.push((id.clone(), title, icon, pages));
    }
    if built.is_empty() { return; }
    let mut ordered: Vec<(Identifier, String, Option<BookIcon>, Vec<BookPage>)> = Vec::new();
    for name in &order {
        let id = if name.contains(':') { Identifier::parse(name).ok() } else { Identifier::new("tconstruct", name).ok() };
        let Some(id) = id else { continue };
        if let Some(position) = built.iter().position(|(entry, ..)| *entry == id) { ordered.push(built.remove(position)); }
    }
    built.sort_by(|a, b| a.1.cmp(&b.1));
    ordered.append(&mut built);
    let groups: Vec<(Option<BookIcon>, String, usize)> = ordered.iter().map(|(_, title, icon, pages)| (icon.clone(), title.clone(), pages.len())).collect();
    let mut generated = overview_pages(&section.title, lookup(table, &format!("{}.subtext", section.name)), &groups);
    for (_, _, _, pages) in ordered { generated.extend(pages); }
    let mut pages: Vec<RawPage> = generated.into_iter().map(|page| RawPage { name: None, kind: None, data: None, extra: serde_json::Map::new(), pages: vec![page], content: None }).collect();
    pages.append(&mut section.pages);
    section.pages = pages;
}

/* The grouping transformers (ToolSectionTransformer / ModifierSectionTransformer) */

#[derive(Clone, Copy, PartialEq)]
enum GroupKind { Tools, Modifiers }

/// The sections each book groups, by index name: every book groups its armor tools; the rest are
/// TinkerBook's per-book transformer registrations.
fn grouping_for(book: &str) -> Vec<(&'static str, GroupKind)> {
    let mut groups = vec![("armor", GroupKind::Tools)];
    match book {
        "materials_and_you" => groups.push(("tools", GroupKind::Tools)),
        "puny_smelting" => { groups.push(("upgrades", GroupKind::Modifiers)); groups.push(("slotless", GroupKind::Modifiers)); }
        "mighty_smelting" => { groups.push(("tools", GroupKind::Tools)); groups.push(("defense", GroupKind::Modifiers)); groups.push(("abilities", GroupKind::Modifiers)); }
        "tinkers_gadgetry" => { groups.push(("staffs", GroupKind::Tools)); groups.push(("ancient_tools", GroupKind::Tools)); }
        "encyclopedia" => {
            groups.push(("tools", GroupKind::Tools));
            groups.push(("upgrades", GroupKind::Modifiers));
            groups.push(("defense", GroupKind::Modifiers));
            groups.push(("slotless", GroupKind::Modifiers));
            groups.push(("abilities", GroupKind::Modifiers));
        }
        _ => {}
    }
    groups
}

/// One listing entry: a group header, or a page link by title.
enum Listing {
    Header(String),
    Page(String, usize),
}

/// Runs one section's grouping transformer: the section's pages are walked in order — tool pages whose
/// item does not exist and modifier pages whose modifier is the default are dropped, a `group_` page
/// opens a group (kept only when it has data of its own), a `hidden` page is kept unlisted, and every
/// other page is listed — and a listing page of the groups and links goes in at the head. The listing's
/// shape follows the original's own listing content: group headers and one link per listed page.
fn grouping_transform(section: &RawSection, table: &[(String, String)], data: &GenData, kind: GroupKind) -> Vec<BookPage> {
    let mut listing: Vec<Listing> = Vec::new();
    let mut kept: Vec<BookPage> = Vec::new();
    for raw in &section.pages {
        let dropped = match (&raw.content, kind) {
            (Some(GenPage::Tool(item)), GroupKind::Tools) => data.items.as_ref().is_some_and(|items| !items.contains(item)),
            (Some(GenPage::Modifier(id)), GroupKind::Modifiers) => data.modifiers.as_ref().is_some_and(|modifiers| is_default_modifier(modifiers, id)),
            _ => false,
        };
        if dropped { continue; }
        let name = raw.name.as_deref().unwrap_or_default();
        if name.starts_with("group_") {
            let title = raw.title().map(str::to_owned)
                .or_else(|| lookup(table, &format!("{}.{}", section.name, name)).map(str::to_owned))
                .unwrap_or_else(|| name.to_owned());
            listing.push(Listing::Header(title));
            if raw.data.as_deref().is_some_and(|data| !data.is_empty() && data != "no-load") {
                kept.extend(raw.pages.iter().cloned());
            }
            continue;
        }
        if name == "hidden" { kept.extend(raw.pages.iter().cloned()); continue; }
        if !raw.pages.is_empty() {
            let title = raw.title().map(str::to_owned)
                .or_else(|| lookup(table, &format!("{}.{}", section.name, name)).map(str::to_owned))
                .unwrap_or_else(|| name.to_owned());
            listing.push(Listing::Page(title, kept.len()));
        }
        kept.extend(raw.pages.iter().cloned());
    }
    if listing.is_empty() { return kept; }
    let rows: Vec<BookRow> = listing.iter().map(|listed| match listed {
        Listing::Header(title) => BookRow::Header { icon: None, text: title.clone() },
        Listing::Page(title, position) => book_row(None, title.clone(), None, Some(*position)),
    }).collect();
    let mut pages = paginate("mantle:index", Some(section.title.clone()), Vec::new(), rows, Vec::new());
    let count = pages.len();
    shift_links(&mut pages, count);
    pages.extend(kept);
    pages
}

/* Building the raw book */

/// A page kept as its decoded form when the content that would generate it cannot (its registry is
/// not loaded, or its subject does not resolve): PlutoLib's decoding of the same JSON.
fn opaque(kind: &str, json: &Value) -> Vec<BookPage> {
    match BookPage::from_json(kind, json) {
        Ok(page) => vec![page],
        Err(error) => { eprintln!("Tinkers: book page of kind {kind} does not decode: {error}"); Vec::new() }
    }
}

/// Mantle's index transformer for a file section: a `mantle:index` page becomes a listing of the
/// section's own pages — each named page's title, linking to its section-local position — skipping
/// the pages its `hidden` list names and itself. The links are section-local here; `build_book`'s
/// shift makes them the book's flat indices, as for every generated listing. Runs before the other
/// transformers: the sections that carry an index page carry no injections or groupings, so their
/// page positions no longer move afterwards. The page's own group operations (headers and column
/// breaks over the listing) are not applied; the listing is flat.
fn index_transform(section: &mut RawSection) {
    let mut flat = 0;
    let mut positions: Vec<usize> = Vec::new();
    for raw in &section.pages { positions.push(flat); flat += raw.pages.len(); }
    let mut built: Vec<(usize, Vec<BookPage>)> = Vec::new();
    for (index, raw) in section.pages.iter().enumerate() {
        if raw.kind.as_deref() != Some("mantle:index") { continue; }
        let Some(BookPage::Opaque { data, .. }) = raw.pages.first() else { continue };
        let title = data.get("title").and_then(Nbt::as_str).map(str::to_owned);
        let subtext = data.get("subText").and_then(Nbt::as_str).map(str::to_owned);
        let hidden: Vec<&str> = match data.get("hidden") {
            Some(Nbt::List(list)) => list.iter().filter_map(Nbt::as_str).collect(),
            _ => Vec::new(),
        };
        let mut rows = Vec::new();
        for (other, other_raw) in section.pages.iter().enumerate() {
            if other == index { continue; }
            let Some(name) = &other_raw.name else { continue };
            if hidden.contains(&name.as_str()) { continue; }
            let label = other_raw.title().map(str::to_owned).unwrap_or_else(|| name.clone());
            rows.push(book_row(None, label, None, Some(positions[other])));
        }
        let intro = subtext.map(|text| vec![pluto_lib::BookText::plain(&text)]).unwrap_or_default();
        built.push((index, paginate("mantle:index", title, intro, rows, Vec::new())));
    }
    for (index, pages) in built { section.pages[index].pages = pages; }
}

/// One ingredient of a captured crafting recipe: an item, or a tag (resolved through the item
/// snapshot when a book builds, as the tag's first member).
#[derive(Clone, Debug)]
enum BookIngredient {
    Item(Identifier),
    Tag(Identifier),
}

/// The crafting recipes the book pages show, captured from the server data reload's recipes
/// folder: recipe id to its nine grid cells (each cell's options) and its result.
static BOOK_RECIPES: std::sync::OnceLock<std::sync::RwLock<std::collections::HashMap<String, (Vec<Vec<BookIngredient>>, Option<(Identifier, u32)>)>>> = std::sync::OnceLock::new();

fn book_recipes() -> &'static std::sync::RwLock<std::collections::HashMap<String, (Vec<Vec<BookIngredient>>, Option<(Identifier, u32)>)>> {
    BOOK_RECIPES.get_or_init(|| std::sync::RwLock::new(std::collections::HashMap::new()))
}

/// One ingredient JSON value's options: an item, a tag, or an array of either. The shipped form
/// writes an ingredient as a bare string (`"minecraft:stick"`, or `"#minecraft:planks"` for a tag);
/// the object form names `item` or `tag`.
fn book_ingredient(value: &Value) -> Vec<BookIngredient> {
    match value {
        Value::Array(list) => list.iter().flat_map(book_ingredient).collect(),
        Value::String(text) => match text.strip_prefix('#') {
            Some(tag) => Identifier::parse(tag).ok().map(|tag| vec![BookIngredient::Tag(tag)]).unwrap_or_default(),
            None => Identifier::parse(text).ok().map(|item| vec![BookIngredient::Item(item)]).unwrap_or_default(),
        },
        Value::Object(_) => {
            if let Some(item) = value.get("item").and_then(Value::as_str).and_then(|id| Identifier::parse(id).ok()) { return vec![BookIngredient::Item(item)]; }
            if let Some(tag) = value.get("tag").and_then(Value::as_str).and_then(|id| Identifier::parse(id).ok()) { return vec![BookIngredient::Tag(tag)]; }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Captures the shaped and shapeless crafting recipes of a data reload for the book pages.
fn capture_crafting_recipes(files: &[(Identifier, Value)]) {
    let mut captured = std::collections::HashMap::new();
    for (id, value) in files {
        let Some(kind) = value.get("type").and_then(Value::as_str) else { continue };
        let result = value.get("result").and_then(|result| {
            let item = match result {
                Value::String(item) => Identifier::parse(item).ok(),
                Value::Object(_) => result.get("id").or_else(|| result.get("item")).and_then(Value::as_str).and_then(|item| Identifier::parse(item).ok()),
                _ => None,
            }?;
            let count = result.get("count").and_then(Value::as_u64).unwrap_or(1);
            Some((item, u32::try_from(count).unwrap_or(1)))
        });
        let mut cells: Vec<Vec<BookIngredient>> = vec![Vec::new(); 9];
        match kind {
            "minecraft:crafting_shaped" => {
                let Some(pattern) = value.get("pattern").and_then(Value::as_array) else { continue };
                let Some(key) = value.get("key").and_then(Value::as_object) else { continue };
                for (row, line) in pattern.iter().take(3).enumerate() {
                    let Some(line) = line.as_str() else { continue };
                    for (column, cell) in line.chars().take(3).enumerate() {
                        if cell == ' ' { continue; }
                        if let Some(ingredient) = key.get(&cell.to_string()) { cells[row * 3 + column] = book_ingredient(ingredient); }
                    }
                }
            }
            "minecraft:crafting_shapeless" => {
                let Some(ingredients) = value.get("ingredients").and_then(Value::as_array) else { continue };
                for (index, ingredient) in ingredients.iter().take(9).enumerate() { cells[index] = book_ingredient(ingredient); }
            }
            _ => continue,
        }
        captured.insert(id.to_string(), (cells, result));
    }
    if let Ok(mut held) = book_recipes().write() { *held = captured; }
}

/// Fills a decoded crafting page's grid and result from the captured recipes, when the capture
/// has the page's recipe (tag ingredients resolve to the tag's first member in the item snapshot).
fn fill_crafting(page: &mut BookPage, data: &GenData) {
    let BookPage::Crafting { recipe, ingredients, result, .. } = page else { return };
    let Ok(held) = book_recipes().read() else { return };
    let Some((cells, captured_result)) = held.get(&recipe.to_string()) else { return };
    *ingredients = cells.iter().map(|cell| cell.iter().filter_map(|ingredient| match ingredient {
        BookIngredient::Item(item) => Some(pluto_lib::ItemSlot::new(item.clone(), 1)),
        BookIngredient::Tag(tag) => data.items.as_ref().and_then(|items| items.get_tag(tag)).and_then(|members| members.iter().next()).map(|item| pluto_lib::ItemSlot::new(item.clone(), 1)),
    }).collect()).collect();
    *result = captured_result.as_ref().map(|(item, count)| pluto_lib::ItemSlot::new(item.clone(), *count));
}

/// Builds one page entry of a section file: blanks and padding markers become blank pages; a data
/// file's generated kinds (tool, modifier, the material contents) build their content when their
/// registry is at hand and ride as their decoded form otherwise; every other kind decodes as before.
/// A page whose file is in neither the reader's language nor English is left out (Mantle logs and
/// skips it too).
fn build_raw_page(read: &Read<'_>, book: &str, language: &str, entry: &Value, data: &GenData) -> RawPage {
    let mut raw = RawPage::new(entry);
    let Some(kind) = raw.kind.clone() else { return raw };
    if kind == "mantle:blank" || kind == "mantle:right_padding" {
        raw.pages.push(BookPage::Text { title: None, text: Vec::new() });
        return raw;
    }
    let Some(path) = raw.data.clone() else { return raw };
    if path.is_empty() || path == "no-load" { return raw; }
    let Some(json) = read_json(read, &format!("book/{book}/{language}/{path}")).or_else(|| read_json(read, &format!("book/{book}/en_us/{path}"))) else {
        eprintln!("Tinkers: book {book}'s page {path} is not in the resources");
        return raw;
    };
    let named_id = |field: &str| json.get(field).and_then(Value::as_str).and_then(|id| Identifier::parse(id).ok())
        .or_else(|| raw.name.as_deref().and_then(|name| Identifier::parse(name).ok()));
    match kind.as_str() {
        "mantle:structure" => {
            // The page's JSON names its template in `data` (a resource id in this namespace, such as
            // `tconstruct:book/structures/smeltery.nbt`); the template itself is a binary NBT file in
            // the same resources, read and decoded here so the page carries its scene from the start.
            raw.pages = opaque(&kind, &json);
            for page in &mut raw.pages {
                let BookPage::Structure { data: id, structure, .. } = page else { continue };
                let Some(path) = id.to_string().split_once(':').map(|(_, path)| path.to_owned()) else { continue };
                let Some(bytes) = read(&path) else {
                    eprintln!("Tinkers: book {book}'s structure {path} is not in the resources");
                    continue;
                };
                match Nbt::from_binary(&bytes).and_then(|root| BookStructure::from_template(&root)) {
                    Ok(filled) => *structure = Some(filled),
                    Err(error) => eprintln!("Tinkers: book {book}'s structure {path} does not read: {error}"),
                }
            }
        }
        "tconstruct:tool" => match named_id("tool") {
            Some(item) => { raw.pages = tool_pages(&item, Some(&json)); raw.content = Some(GenPage::Tool(item)); }
            None => raw.pages = opaque(&kind, &json),
        },
        "tconstruct:modifier" => {
            let id = named_id("modifier_id");
            match (id, &data.modifiers) {
                (Some(id), Some(modifiers)) => {
                    raw.content = Some(GenPage::Modifier(id.clone()));
                    raw.pages = modifier_pages(modifiers, &id, Some(&json)).unwrap_or_else(|| opaque(&kind, &json));
                }
                _ => raw.pages = opaque(&kind, &json),
            }
        }
        _ => {
            if let Some((_, kind_type)) = MATERIAL_TYPES.iter().find(|(_, material)| material.kind == kind) {
                let material = json.get("material").and_then(Value::as_str).and_then(|id| Identifier::parse(id).ok());
                match (material, &data.materials) {
                    (Some(material), Some(definitions)) => {
                        let title = json.get("title").and_then(Value::as_str).filter(|title| !title.is_empty()).map(str::to_owned);
                        let detailed = json.get("detailed").and_then(Value::as_bool).unwrap_or(false);
                        raw.pages = material_pages(definitions, data, kind_type, &material, detailed, title, None);
                    }
                    _ => raw.pages = opaque(&kind, &json),
                }
            } else {
                raw.pages = opaque(&kind, &json);
                if kind == "mantle:crafting" {
                    for page in &mut raw.pages { fill_crafting(page, data); }
                }
            }
        }
    }
    raw
}

/// Builds one book from its Mantle data in the reader's language, generating the transformer content
/// from `data`'s registries. A section left with no pages is left out (the registry takes none),
/// except a generated section, which keeps its index page.
pub(crate) fn build_book(read: &Read<'_>, name: &str, language: &str, data: &GenData) -> Result<Book, String> {
    let root = format!("book/{name}");
    let index = read_json(read, &format!("{root}/index.json")).ok_or_else(|| format!("book {name} has no index"))?;
    let entries = index.as_array().ok_or_else(|| format!("book {name}'s index is not a section list"))?;
    let appearance = read_json(read, &format!("{root}/{language}/appearance.json"))
        .or_else(|| read_json(read, &format!("{root}/appearance.json")))
        .map(|data| BookAppearance::from_json(&data)).transpose()?.unwrap_or_default();
    let table = language_table(read, name, language);
    let mut sections: Vec<RawSection> = Vec::new();
    for entry in entries {
        let Some(section_name) = entry.get("name").and_then(Value::as_str) else { continue };
        if let Some(condition) = entry.get("condition") {
            if !section_condition_met(condition, data) { continue; }
        }
        let title = lookup(&table, section_name).unwrap_or(section_name).to_owned();
        let icon = entry.get("icon").and_then(icon);
        let extra = entry.get("extraData").and_then(Value::as_object).cloned().unwrap_or_default();
        let path = entry.get("data").and_then(Value::as_str);
        let generated = path.is_none() || path == Some("") || path == Some("no-load");
        let mut section = RawSection { name: section_name.to_owned(), title, icon, extra, generated, pages: Vec::new() };
        if !generated {
            let refs = read_json(read, &format!("{root}/{}", path.unwrap_or_default()));
            match refs.as_ref().and_then(Value::as_array) {
                Some(refs) => section.pages = refs.iter().map(|entry| build_raw_page(read, name, language, entry, data)).collect(),
                None => eprintln!("Tinkers: book {name}'s section {section_name} does not load"),
            }
            index_transform(&mut section);
        }
        sections.push(section);
    }
    let grouping = grouping_for(name);
    let mut flat = 0;
    let mut built: Vec<pluto_lib::BookSection> = Vec::new();
    for mut section in sections {
        inject_section(read, name, language, &mut section, data);
        tier_transform(&mut section, &table, data);
        fluid_transform(read, name, language, &mut section, &table, data);
        let kind = grouping.iter().find(|(grouped, _)| *grouped == section.name).map(|(_, kind)| *kind);
        let mut pages: Vec<BookPage> = match kind {
            Some(kind) => grouping_transform(&section, &table, data, kind),
            None => section.pages.into_iter().flat_map(|raw| raw.pages).collect(),
        };
        if pages.is_empty() {
            if section.generated {
                let subtext = lookup(&table, &format!("{}.subtext", section.name));
                pages.push(index_page(&section.title, subtext));
            } else {
                eprintln!("Tinkers: book {name}'s section {} has no pages and is left out", section.name);
                continue;
            }
        }
        shift_links(&mut pages, flat);
        flat += pages.len();
        built.push(pluto_lib::BookSection { name: section.title, icon: section.icon, pages });
    }
    let id = Identifier::new("tconstruct", name).map_err(|error| error.to_string())?;
    Ok(Book { id, appearance, sections: built })
}

/// Builds every book from the active client resources and the current registries, replacing the
/// registered books: a rebuild after the registries arrive supersedes the first build's index pages
/// with the generated content. A book that is open keeps the copy it opened with.
fn rebuild_books() {
    let read = |path: &str| {
        let id = Identifier::new("tconstruct", path).ok()?;
        ResourceManagerHelper::read_client_resource(&id)
    };
    let language = I18n::language();
    let data = gather_gen_data();
    for name in BOOKS {
        match build_book(&read, name, &language, &data) {
            Ok(book) => if let Err(error) = BookRegistry::replace(book) { eprintln!("Tinkers: book {name} not registered: {error}"); },
            Err(error) => eprintln!("Tinkers: {error}"),
        }
    }
}

fn load_books() { rebuild_books(); }

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
    // The crafting pages' grids come from the crafting recipes themselves, captured at the data
    // reload; the books rebuild afterwards through the same triggers as the other registries.
    let _ = ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "book_recipes").expect("valid listener id"), "recipe",
        |files| { capture_crafting_recipes(files); Ok(()) });
    // The generated pages come from the registries; when the materials land (a local load or a remote
    // snapshot), the books rebuild with them.
    crate::materials::MATERIALS_LOADED.register(|_| rebuild_books());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::{BTreeMap, HashMap};

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
        let book = build_book(&read, "materials_and_you", "en_us", &GenData::default()).unwrap();
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
        assert_eq!(materials.pages.len(), 1, "a generated section without its registries keeps its index page only");
        assert_eq!(materials.pages[0].kind(), "mantle:index");
        assert_eq!(materials.pages[0].title(), Some("Tier 1 Materials"));
        let tools = &book.sections[2];
        assert_eq!(tools.pages.len(), 2, "the grouping listing page and the file page");
        let BookPage::Table { title, rows, .. } = &tools.pages[0] else { panic!("the tools section leads with its listing page") };
        assert_eq!(title.as_deref(), Some("Tools"));
        assert!(matches!(&rows[0], BookRow::Header { text, .. } if text == "General"), "the group anchor opens its group in the listing");
        assert!(matches!(&rows[1], BookRow::Entry { label, link, .. } if label == "Repairing" && *link == Some(5)),
            "the file page is listed with a link to its flat page index (3 intro + 1 materials + 1 listing)");
        assert_eq!(tools.pages[1].title(), Some("Repairing"));
        assert_eq!(book.page_count(), 6);
    }

    #[test]
    fn a_page_falls_back_to_english_and_a_missing_page_is_left_out() {
        let mut files = fixture();
        files.remove("book/materials_and_you/en_us/tools/repairing.json");
        let read = |path: &str| files.get(path).cloned();
        let book = build_book(&read, "materials_and_you", "ja_jp", &GenData::default()).unwrap();
        assert_eq!(book.sections[0].pages.len(), 3, "the English pages stand in for the missing translation");
        assert_eq!(book.sections[2].pages.len(), 1, "only the listing page remains of the tools section");
    }

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    /// A small material registry: rock (tier 1, head stats, a book trait), flint (tier 1, handle
    /// stats only) and steel (tier 3, head stats) — enough to exercise the tier filter, the melee
    /// sort and the trait lines.
    fn test_definitions() -> Definitions {
        let mut definitions = Definitions {
            materials: BTreeMap::new(), redirects: BTreeMap::new(), visible: Vec::new(),
            tags: BTreeMap::new(), reverse_tags: BTreeMap::new(), traits: BTreeMap::new(),
            stats: BTreeMap::new(), server_loaded: true, generation: 1, snapshot: None, problems: Vec::new(),
        };
        for (mid, tier) in [("tconstruct:rock", 1), ("tconstruct:flint", 1), ("tconstruct:steel", 3)] {
            let mid = id(mid);
            definitions.visible.push(mid.clone());
            definitions.materials.insert(mid.clone(), crate::materials::Material {
                id: mid, tier, sort_order: 0, rarity: crate::materials::Rarity::Common, craftable: true, hidden: false,
            });
        }
        let head = || Stats::Head { durability: 100, mining_speed: 2.0, tier: id("minecraft:stone"), attack: 1.0 };
        definitions.stats.insert(id("tconstruct:rock"), BTreeMap::from([(tid("head"), head())]));
        definitions.stats.insert(id("tconstruct:steel"), BTreeMap::from([(tid("head"), head())]));
        definitions.stats.insert(id("tconstruct:flint"), BTreeMap::from([(tid("handle"),
            Stats::Handle { durability: 0.9, mining_speed: 1.0, melee_speed: 1.0, attack: 0.0 })]));
        definitions.traits.insert(id("tconstruct:rock"), crate::material_traits::MaterialTraits {
            default: Vec::new(),
            per_stat: BTreeMap::from([(tid("head"), vec![Some(crate::material_traits::ModifierEntry {
                id: id("tconstruct:sharp"), level: 1, optional: false,
            })])]),
        });
        definitions.tags.insert(tid("book/distant"), Vec::new());
        definitions
    }

    fn test_modifiers() -> crate::modifiers::Registry {
        let mut registry = crate::modifiers::Registry::default();
        registry.register_static(Modifier::decode(id("tconstruct:sharp"), &json!({})).unwrap()).unwrap();
        registry
    }

    #[test]
    fn a_material_section_generates_an_overview_and_a_page_per_material() {
        let mut files = HashMap::new();
        let mut put = |path: &str, value: Value| { files.insert(format!("book/materials_and_you/{path}"), serde_json::to_vec(&value).unwrap()); };
        put("index.json", json!([
            {"name": "materials", "data": "no-load", "extraData": {"tconstruct:material_tier": {
                "type": "tconstruct:melee_harvest", "tier": {"min": 0, "max": 1},
                "predicate": {"type": "mantle:inverted", "inverted_type": "mantle:tag", "tag": "tconstruct:book/distant"}}}}
        ]));
        files.insert("book/materials_and_you/en_us/language.lang".to_owned(), b"materials=Materials\nmaterials.subtext=Pick one.\n".to_vec());
        let read = |path: &str| files.get(path).cloned();
        let data = GenData { materials: Some(test_definitions()), modifiers: Some(test_modifiers()), ..GenData::default() };
        let book = build_book(&read, "materials_and_you", "en_us", &data).unwrap();
        let section = &book.sections[0];
        assert_eq!(section.pages.len(), 3, "the overview and the two tier 1 materials (steel is tier 3)");
        let BookPage::Table { kind, rows, .. } = &section.pages[0] else { panic!("the section leads with its overview") };
        assert_eq!(kind, "mantle:index");
        assert_eq!(rows.len(), 2);
        assert!(matches!(&rows[0], BookRow::Entry { label, link, .. } if label == "material.tconstruct.rock" && *link == Some(1)),
            "rock (with a head) lists first and links to its page");
        assert!(matches!(&rows[1], BookRow::Entry { label, link, .. } if label == "material.tconstruct.flint" && *link == Some(2)));
        let BookPage::Table { kind, title, rows, .. } = &section.pages[1] else { panic!("a material page is a table") };
        assert_eq!(kind, "tconstruct:melee_harvest_material");
        assert_eq!(title.as_deref(), Some("material.tconstruct.rock"), "no translations are registered in a test, so keys stand");
        assert!(matches!(&rows[0], BookRow::Header { text, .. } if text == "stat.tconstruct.head"));
        assert!(rows.iter().any(|row| matches!(row, BookRow::Entry { label, .. } if label == "modifier.tconstruct.sharp")),
            "the head trait shows by its display name");
        let BookPage::Table { rows, .. } = &section.pages[2] else { panic!("a material page is a table") };
        assert!(matches!(&rows[0], BookRow::Header { text, .. } if text == "stat.tconstruct.handle"), "flint has handle stats only");
    }

    #[test]
    fn modifier_pages_generate_behind_a_group_anchor() {
        let mut files = HashMap::new();
        let mut put = |path: &str, value: Value| { files.insert(format!("book/puny_smelting/{path}"), serde_json::to_vec(&value).unwrap()); };
        put("index.json", json!([{"name": "upgrades", "data": "sections/upgrades.json"}]));
        put("sections/upgrades.json", json!([
            {"name": "group_general", "type": "mantle:text", "data": "upgrades/general.json",
             "extraData": {"tconstruct:load_modifiers": {"tag": "tconstruct:upgrades/general", "path": "upgrades/general", "prefix": "general", "sort": false}}},
            {"name": "afterword", "type": "mantle:text", "data": "upgrades/afterword.json"}
        ]));
        put("en_us/upgrades/general.json", json!({"title": "General Upgrades", "text": [{"text": "About upgrades."}]}));
        put("en_us/upgrades/general/tconstruct_sharp.json", json!({"text": [{"text": "Sharper."}], "effects": ["Cuts deeper"]}));
        put("en_us/upgrades/afterword.json", json!({"title": "Afterword", "text": [{"text": "Done."}]}));
        files.insert("book/puny_smelting/en_us/language.lang".to_owned(), b"upgrades=Upgrades\n".to_vec());
        let read = |path: &str| files.get(path).cloned();
        let mut modifiers = test_modifiers();
        modifiers.tags.insert(id("tconstruct:upgrades/general"), vec![id("tconstruct:sharp")]);
        let data = GenData { modifiers: Some(modifiers), ..GenData::default() };
        let book = build_book(&read, "puny_smelting", "en_us", &data).unwrap();
        let section = &book.sections[0];
        assert_eq!(section.pages.len(), 4, "listing, the group page, the injected modifier page, the file page");
        let BookPage::Table { rows, .. } = &section.pages[0] else { panic!("the section leads with its listing") };
        assert!(matches!(&rows[0], BookRow::Header { text, .. } if text == "General Upgrades"), "the group is titled by its page");
        assert!(matches!(&rows[1], BookRow::Entry { label, link, .. } if label == "modifier.tconstruct.sharp" && *link == Some(2)));
        assert!(matches!(&rows[2], BookRow::Entry { label, link, .. } if label == "Afterword" && *link == Some(3)));
        assert_eq!(section.pages[1].title(), Some("General Upgrades"), "a group page with data is kept");
        let BookPage::Table { kind, rows, .. } = &section.pages[2] else { panic!("a modifier page is a table") };
        assert_eq!(kind, "tconstruct:modifier");
        assert!(rows.iter().any(|row| matches!(row, BookRow::Entry { label, .. } if label == "● Cuts deeper")), "its effects list, bulleted");
    }

    #[test]
    fn a_fluid_section_generates_its_pages_in_the_specs_order() {
        let mut files = HashMap::new();
        let mut put = |path: &str, value: Value| { files.insert(format!("book/encyclopedia/{path}"), serde_json::to_vec(&value).unwrap()); };
        put("index.json", json!([
            {"name": "fluid_effects", "data": "no-load", "extraData": {"tconstruct:fluid_effects": {"path": "fluids", "order": ["water", "zzz"]}}}
        ]));
        put("en_us/fluids/tconstruct_water.json", json!({"title": "Water", "text": [{"text": "Wet."}], "block": ["Soaks the ground"]}));
        files.insert("book/encyclopedia/en_us/language.lang".to_owned(), b"fluid_effects=Fluid Effects\n".to_vec());
        let read = |path: &str| files.get(path).cloned();
        let effect_entry = |fluid: &str, effects: Value| FluidEffects::decode(&effects).map(|decoded| (id(fluid), decoded)).unwrap();
        let data = GenData { fluid_effects: Some(vec![
            effect_entry("tconstruct:molten_iron", json!({"fluid": {"fluid": "tconstruct:molten_iron", "amount": 1000}, "entity_effects": [{"type": "tconstruct:extinguish"}]})),
            effect_entry("tconstruct:water", json!({"fluid": {"fluid": "minecraft:water", "amount": 1000}})),
        ]), ..GenData::default() };
        let book = build_book(&read, "encyclopedia", "en_us", &data).unwrap();
        let section = &book.sections[0];
        assert_eq!(section.pages.len(), 3, "the overview and one page per entry");
        let BookPage::Table { rows, .. } = &section.pages[0] else { panic!("the section leads with its overview") };
        assert!(matches!(&rows[0], BookRow::Entry { label, link, .. } if label == "Water" && *link == Some(1)),
            "the order list puts water first (its unknown zzz is skipped)");
        assert!(matches!(&rows[1], BookRow::Entry { label, .. } if label == "fluid.tconstruct.molten_iron"));
        assert_eq!(section.pages[1].title(), Some("Water"), "the data file's title wins");
        let BookPage::Table { rows, .. } = &section.pages[1] else { panic!("a fluid page is a table") };
        assert!(rows.iter().any(|row| matches!(row, BookRow::Entry { label, .. } if label == "● Soaks the ground")),
            "the data file's block list wins over generation");
        let BookPage::Table { kind, rows, .. } = &section.pages[2] else { panic!("a fluid page is a table") };
        assert_eq!(kind, "tconstruct:fluid_effect");
        assert!(rows.iter().any(|row| matches!(row, BookRow::Header { text, .. } if text == "book.tconstruct.fluid_effects.entity")));
        assert!(rows.iter().any(|row| matches!(row, BookRow::Entry { label, .. } if label == "● fluid_effect.tconstruct.extinguish")),
            "the entity list generates from the entry's effects");
    }

    #[test]
    fn a_crafting_page_fills_its_grid_from_the_captured_recipes() {
        capture_crafting_recipes(&[(Identifier::parse("tconstruct:tables/book_test_pattern").unwrap(), json!({
            "type": "minecraft:crafting_shaped",
            "pattern": ["ps", "sp"],
            "key": {"p": "#minecraft:planks", "s": "minecraft:stick"},
            "result": {"id": "tconstruct:pattern", "count": 6},
        }))]);
        let mut page = BookPage::from_json("mantle:crafting", &json!({"title": "Pattern", "recipe": "tconstruct:tables/book_test_pattern"})).unwrap();
        fill_crafting(&mut page, &GenData::default());
        match &page {
            BookPage::Crafting { ingredients, result, .. } => {
                assert_eq!(ingredients.len(), 9);
                assert!(ingredients[0].is_empty(), "the tag cell stays empty without an item snapshot");
                assert_eq!(ingredients[1].len(), 1);
                assert_eq!(ingredients[1][0].item.to_string(), "minecraft:stick");
                assert!(ingredients[8].is_empty(), "the third row stays empty");
                assert_eq!(result.as_ref().map(|stack| (stack.item.to_string(), stack.count)), Some(("tconstruct:pattern".to_owned(), 6)));
            }
            other => panic!("a crafting page: {other:?}"),
        }
    }

    #[test]
    fn an_index_page_lists_its_sections_pages_and_skips_the_hidden_ones() {
        let mut files = fixture();
        files.insert("book/materials_and_you/sections/introduction.json".to_owned(), serde_json::to_vec(&json!([
            {"name": "index", "type": "mantle:index", "data": "intro/index.json"},
            {"name": "welcome", "type": "mantle:text", "data": "intro/welcome.json"},
            {"name": "craft", "type": "mantle:crafting", "data": "intro/pattern_craft.json"}
        ])).unwrap());
        files.insert("book/materials_and_you/en_us/intro/index.json".to_owned(), serde_json::to_vec(&json!({
            "title": "Contents", "subText": "Start here.", "hidden": ["craft"]
        })).unwrap());
        let read = |path: &str| files.get(path).cloned();
        let book = build_book(&read, "materials_and_you", "en_us", &GenData::default()).unwrap();
        let first = &book.sections[0].pages[0];
        match first {
            BookPage::Table { kind, rows, .. } => {
                assert_eq!(kind, "mantle:index");
                assert_eq!(rows.len(), 1, "only the welcome page is listed (the craft page is hidden, the index itself is not): {rows:?}");
                match &rows[0] {
                    BookRow::Entry { label, link, .. } => {
                        assert_eq!(label, "Materials & You");
                        assert_eq!(*link, Some(1), "the welcome page is the section's second page");
                    }
                    other => panic!("a listing row: {other:?}"),
                }
            }
            other => panic!("the index page became a listing, not {other:?}"),
        }
    }

    #[test]
    fn an_armor_section_condition_keeps_the_section_without_a_registry() {
        let mut files = fixture();
        files.insert("book/materials_and_you/index.json".to_owned(), serde_json::to_vec(&json!([
            {"name": "armor", "data": "no-load", "extraData": {"tconstruct:load_tools": {"tag": "tconstruct:modifiable/book_armor/materials_and_you", "path": "armor", "sort": false}},
             "condition": {"type": "tconstruct:tag_not_empty", "registry": "minecraft:item", "tag": "tconstruct:modifiable/book_armor/materials_and_you"}}
        ])).unwrap());
        let read = |path: &str| files.get(path).cloned();
        let book = build_book(&read, "materials_and_you", "en_us", &GenData::default()).unwrap();
        assert_eq!(book.sections.len(), 1, "without a captured item registry the conditioned section is kept");
        assert_eq!(book.sections[0].pages.len(), 1, "and keeps its index page");
    }
}
