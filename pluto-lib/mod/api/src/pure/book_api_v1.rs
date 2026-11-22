//! Books a reader opens on the client (Mantle: `mantle:book`): a mod registers a book's sections and pages once, and any
//! callback that carries a player can ask for the book to open. The content model follows the original book data format
//! (an index of sections, an appearance, and pages of text, image-and-text and crafting kinds, plus table pages a mod
//! builds from its registries — a row per material, tool or modifier, say); the drawing and the page
//! turning are done here, over the same [`ScreenCanvas`] the menu screens draw on, so they can be tested without the game.
//!
//! Opening is a client act: the host opens PlutoLib's own mod screen and the screen hooks draw the current page into it.
//! A callback on the server thread cannot open a client screen (the host says so and [`open_book`] answers false); a mod
//! whose book item is used on the server tells the client over its own channel (see `networking_api_v1`) and the client
//! side calls [`open_book`] there.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::Mutex;

use super::base::{Identifier, Nbt, NbtCompound};
use super::block_entity_api_v1::ItemSlot;
use super::screen_handler_api_v1::ScreenCanvas;

/// One run of text on a page (Mantle: one entry of a page's `text` list). `paragraph` asks for a blank line before it.
#[derive(Clone, Debug, PartialEq)]
pub struct BookText {
    /// The text itself; a translation key or literal text, as the book's data has it (the renderer draws it as given).
    pub text: String,
    /// A blank line is drawn before this run.
    pub paragraph: bool,
}

impl BookText {
    /// A plain run of text, no blank line before it.
    pub fn plain(text: &str) -> Self {
        Self { text: text.to_owned(), paragraph: false }
    }

    /// A run of text that starts a new paragraph.
    pub fn paragraph(text: &str) -> Self {
        Self { text: text.to_owned(), paragraph: true }
    }
}

/// An image on a page (Mantle: a page's `image`): the GUI texture `file` and its size in texture pixels. A page may
/// name no size; the size is then 0 and the renderer leaves the image out (it cannot know the texture's extent).
#[derive(Clone, Debug, PartialEq)]
pub struct BookImage {
    /// The texture's id, such as `tconstruct:book/images/smeltery.png` (drawn through the canvas's sprite surface).
    pub file: String,
    /// The image's width in texture pixels, or 0 when the page names none.
    pub width: u32,
    /// The image's height in texture pixels, or 0 when the page names none.
    pub height: u32,
}

/// How big a crafting page draws its grid (Mantle: a crafting page's `grid_size`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridSize {
    /// The compact grid (`"small"` in the book data).
    #[default]
    Small,
    /// The full-size grid (`"large"` in the book data).
    Large,
}

impl GridSize {
    /// The book data's name for the size.
    pub fn parse(name: &str) -> Self {
        match name {
            "large" => Self::Large,
            _ => Self::Small,
        }
    }

    /// The side of one grid cell in GUI pixels, as this renderer draws it.
    fn cell(self) -> f32 {
        match self {
            Self::Small => 12.0,
            Self::Large => 18.0,
        }
    }
}

/// One row of a [`BookPage::Table`]: a header naming what follows, or one entry of the table. A row is one line
/// tall when drawn; a producer whose table is longer than a page splits it into several table pages itself
/// (the original's contents do the same across a two-page spread).
#[derive(Clone, Debug, PartialEq)]
pub enum BookRow {
    /// A header row (a group title, a stat block's name): drawn underlined, with its icon when it has one.
    Header {
        /// The row's icon, if it has one.
        icon: Option<BookIcon>,
        /// The header's text.
        text: String,
    },
    /// One entry of the table: a label, an optional value drawn after it, and an optional link to another
    /// page of the same book (the flat page index, as the original's index and listing pages link).
    Entry {
        /// The row's icon, if it has one.
        icon: Option<BookIcon>,
        /// The entry's label.
        label: String,
        /// The entry's value, drawn right-aligned on the same line, if it has one.
        value: Option<String>,
        /// The flat index of the page a click on this row opens, if the row is a link.
        link: Option<usize>,
    },
}

/// One page of a book. Text, image-and-text, crafting and table pages are the kinds this renderer draws in full;
/// any other Mantle page kind (a smelting page, ...) is kept as [`BookPage::Opaque`] with its data intact, so a
/// book loses nothing by being registered before its kind has a renderer.
#[derive(Clone, Debug, PartialEq)]
pub enum BookPage {
    /// Text only (Mantle: `mantle:text`).
    Text {
        /// The page's title, if it has one.
        title: Option<String>,
        /// The page's text, in order.
        text: Vec<BookText>,
    },
    /// An image with text under it (Mantle: `mantle:image_text`).
    ImageText {
        /// The page's title, if it has one.
        title: Option<String>,
        /// The page's image.
        image: BookImage,
        /// The page's text, in order.
        text: Vec<BookText>,
    },
    /// A crafting recipe display (Mantle: `mantle:crafting`).
    Crafting {
        /// The page's title, if it has one.
        title: Option<String>,
        /// The recipe the page shows.
        recipe: Identifier,
        /// How big the grid is drawn.
        grid_size: GridSize,
        /// The text under the grid.
        description: Vec<BookText>,
    },
    /// A table of registry entries (a page a mod generates from its registries: one row per material of a
    /// material page, per tool of an index page, ...). The page keeps the `kind` of the content it stands for
    /// (a material page keeps `tconstruct:melee_harvest_material`, say), so a book's pages keep the original's
    /// kinds even though one renderer draws them all.
    Table {
        /// The page kind's id, such as `tconstruct:melee_harvest_material`.
        kind: String,
        /// The page's title, if it has one.
        title: Option<String>,
        /// Text drawn above the rows.
        intro: Vec<BookText>,
        /// The table's rows, in order.
        rows: Vec<BookRow>,
        /// Text drawn below the rows.
        text: Vec<BookText>,
    },
    /// A page kind this renderer does not draw yet; `data` is the page's own data, converted from its JSON.
    Opaque {
        /// The page kind's id, such as `mantle:smelting`.
        kind: String,
        /// The page's data (its localized JSON, as NBT).
        data: NbtCompound,
    },
}

impl BookPage {
    /// The Mantle page kind's id (`mantle:text`, `mantle:crafting`, ...).
    pub fn kind(&self) -> &str {
        match self {
            Self::Text { .. } => "mantle:text",
            Self::ImageText { .. } => "mantle:image_text",
            Self::Crafting { .. } => "mantle:crafting",
            Self::Table { kind, .. } => kind,
            Self::Opaque { kind, .. } => kind,
        }
    }

    /// The page's title, if it has one. An opaque page's title is read from its data's `title` string, if it has one.
    pub fn title(&self) -> Option<&str> {
        match self {
            Self::Text { title, .. } | Self::ImageText { title, .. } | Self::Crafting { title, .. } | Self::Table { title, .. } => title.as_deref(),
            Self::Opaque { data, .. } => data.get("title").and_then(Nbt::as_str),
        }
    }

    /// Decodes one localized page from its JSON (the file the section's page entry names), by its Mantle kind.
    /// Unknown kinds decode to [`BookPage::Opaque`] with the JSON kept as NBT, as does a crafting page in the
    /// literal-grid form (it names no recipe but carries a grid); a known kind with malformed data is an error
    /// that names the field.
    pub fn from_json(kind: &str, data: &serde_json::Value) -> Result<Self, String> {
        let title = data.get("title").and_then(serde_json::Value::as_str).map(str::to_owned);
        match kind {
            "mantle:text" => Ok(Self::Text { title, text: text_list(data.get("text"))? }),
            "mantle:image_text" => {
                let image = data.get("image").ok_or("image_text page has no image")?;
                let file = image.get("file").and_then(serde_json::Value::as_str).ok_or("image_text page's image has no file")?;
                let width = image.get("width").and_then(serde_json::Value::as_u64).unwrap_or(0);
                let height = image.get("height").and_then(serde_json::Value::as_u64).unwrap_or(0);
                Ok(Self::ImageText {
                    title,
                    image: BookImage { file: file.to_owned(), width: u32::try_from(width).map_err(|_| "image width out of range")?, height: u32::try_from(height).map_err(|_| "image height out of range")? },
                    text: text_list(data.get("text"))?,
                })
            }
            "mantle:crafting" => {
                // A crafting page either names a recipe or spells out a literal grid and result instead; the grid
                // form references no recipe, so it is kept whole as an opaque page (its title still reads). A page
                // with neither is malformed, not a grid page.
                let Some(recipe) = data.get("recipe").and_then(serde_json::Value::as_str) else {
                    if data.get("grid").is_some() {
                        return Ok(Self::Opaque { kind: kind.to_owned(), data: json_compound(data) });
                    }
                    return Err("crafting page has no recipe".to_owned());
                };
                Ok(Self::Crafting {
                    title,
                    recipe: Identifier::parse(recipe).map_err(|e| format!("crafting page's recipe: {e}"))?,
                    grid_size: GridSize::parse(data.get("grid_size").and_then(serde_json::Value::as_str).unwrap_or("small")),
                    description: text_list(data.get("description"))?,
                })
            }
            _ => Ok(Self::Opaque { kind: kind.to_owned(), data: json_compound(data) }),
        }
    }
}

/// A page's `text` list: entries of `{text, paragraph?}`; a bare string entry is one plain run.
fn text_list(value: Option<&serde_json::Value>) -> Result<Vec<BookText>, String> {
    let Some(serde_json::Value::Array(entries)) = value else { return Ok(Vec::new()) };
    entries
        .iter()
        .map(|entry| match entry {
            serde_json::Value::String(text) => Ok(BookText::plain(text)),
            serde_json::Value::Object(_) => {
                let text = entry.get("text").and_then(serde_json::Value::as_str).ok_or("a text entry has no text")?;
                Ok(BookText { text: text.to_owned(), paragraph: entry.get("paragraph").and_then(serde_json::Value::as_bool).unwrap_or(false) })
            }
            _ => Err("a text entry is neither text nor an object".to_owned()),
        })
        .collect()
}

/// A JSON value as NBT, for the data an opaque page keeps (objects become compounds, arrays lists, numbers the
/// narrowest number tag that holds them).
fn json_to_nbt(value: &serde_json::Value) -> Nbt {
    match value {
        serde_json::Value::Null => Nbt::String(String::new()),
        serde_json::Value::Bool(b) => Nbt::Byte(i8::from(*b)),
        serde_json::Value::Number(n) => {
            if let Some(v) = n.as_i64() {
                i32::try_from(v).map_or(Nbt::Long(v), Nbt::Int)
            } else {
                Nbt::Double(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Nbt::String(s.clone()),
        serde_json::Value::Array(items) => Nbt::List(items.iter().map(json_to_nbt).collect()),
        serde_json::Value::Object(_) => Nbt::Compound(json_compound(value)),
    }
}

fn json_compound(value: &serde_json::Value) -> NbtCompound {
    match value {
        serde_json::Value::Object(map) => map.iter().map(|(key, value)| (key.clone(), json_to_nbt(value))).collect(),
        _ => NbtCompound::new(),
    }
}

/// What a section's list entry shows (Mantle: a section's `icon`): an item, or an item with data.
#[derive(Clone, Debug, PartialEq)]
pub enum BookIcon {
    /// The item alone (`{"item": "tconstruct:pickaxe"}`).
    Item(Identifier),
    /// The item with the data the index gives it (`{"type": "forge:nbt", "item": ..., "nbt": {...}}`).
    ItemWithData {
        /// The item.
        item: Identifier,
        /// Its data, converted from the index's JSON object.
        data: NbtCompound,
    },
}

impl BookIcon {
    /// Decodes a section's `icon` from the index JSON.
    pub fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        let item = value.get("item").and_then(serde_json::Value::as_str).ok_or("a book icon has no item")?;
        let item = Identifier::parse(item).map_err(|e| format!("a book icon's item: {e}"))?;
        match value.get("nbt") {
            Some(nbt) => Ok(Self::ItemWithData { item, data: json_compound(nbt) }),
            None => Ok(Self::Item(item)),
        }
    }

    /// The icon's item.
    pub fn item(&self) -> &Identifier {
        match self {
            Self::Item(item) => item,
            Self::ItemWithData { item, .. } => item,
        }
    }
}

/// One section of a book (Mantle: one entry of the book's index): a name for the section list and its pages, in order.
#[derive(Clone, Debug, PartialEq)]
pub struct BookSection {
    /// The section's name (a translation key or literal text, as the index has it).
    pub name: String,
    /// The section's icon, if the index gives one.
    pub icon: Option<BookIcon>,
    /// The section's pages, in order.
    pub pages: Vec<BookPage>,
}

/// A page entry of a section file, before its localized page is loaded: the page's name, its Mantle kind and the
/// language-relative path of its page JSON (Mantle: one entry of a section's JSON array).
#[derive(Clone, Debug, PartialEq)]
pub struct SectionPageRef {
    /// The page's name within the section.
    pub name: String,
    /// The page kind's id, such as `mantle:text`.
    pub kind: String,
    /// The path of the page's localized JSON, relative to the book's language folder.
    pub data: String,
}

/// Reads a section file's page entries. The consumer loads each entry's page JSON (in the reader's language)
/// and decodes it with [`BookPage::from_json`].
pub fn section_page_refs(section_json: &serde_json::Value) -> Result<Vec<SectionPageRef>, String> {
    let serde_json::Value::Array(entries) = section_json else { return Err("a book section is not a page list".to_owned()) };
    entries
        .iter()
        .map(|entry| {
            let name = entry.get("name").and_then(serde_json::Value::as_str).ok_or("a page entry has no name")?;
            let kind = entry.get("type").and_then(serde_json::Value::as_str).ok_or("a page entry has no type")?;
            let data = entry.get("data").and_then(serde_json::Value::as_str).ok_or("a page entry has no data")?;
            Ok(SectionPageRef { name: name.to_owned(), kind: kind.to_owned(), data: data.to_owned() })
        })
        .collect()
}

/// How a book looks (Mantle: its `appearance.json`). A colour of 0 means the data did not set it and the
/// renderer uses its own; the texture fields are GUI texture ids the canvas draws as sprites.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BookAppearance {
    /// The cover's texture, if the book has one.
    pub cover_texture: Option<String>,
    /// The cover's colour (ARGB), 0 when unset.
    pub cover_color: u32,
    /// The open book's page texture, if the book has one.
    pub book_texture: Option<String>,
    /// The page arrows' colour (ARGB), 0 when unset.
    pub arrow_color: u32,
    /// The colour behind item slots on pages (ARGB), 0 when unset.
    pub slot_color: u32,
    /// The section list draws its sections' names (Mantle draws icons only when this is false).
    pub draw_section_list_text: bool,
    /// Page titles are drawn large. (The canvas has one text size, so the renderer carries the flag but
    /// cannot scale the title; see the module docs of the renderer below.)
    pub large_page_titles: bool,
    /// Page titles are centered rather than left-aligned.
    pub center_page_titles: bool,
    /// The title the book exports under, if any.
    pub export_title: Option<String>,
}

impl BookAppearance {
    /// Decodes a book's `appearance.json`. Colours are the JSON integers Mantle writes (an ARGB packed in
    /// a signed integer); a missing field keeps its default.
    pub fn from_json(data: &serde_json::Value) -> Result<Self, String> {
        let color = |key: &str| -> Result<u32, String> {
            match data.get(key) {
                None => Ok(0),
                Some(value) => value.as_i64().map(|v| v as u32).ok_or_else(|| format!("appearance {key} is not a number")),
            }
        };
        let texture = |key: &str| data.get(key).and_then(serde_json::Value::as_str).map(str::to_owned);
        let flag = |key: &str| data.get(key).and_then(serde_json::Value::as_bool).unwrap_or(false);
        Ok(Self {
            cover_texture: texture("coverTexture"),
            cover_color: color("coverColor")?,
            book_texture: texture("bookTexture"),
            arrow_color: color("arrowColor")?,
            slot_color: color("slotColor")?,
            draw_section_list_text: flag("drawSectionListText"),
            large_page_titles: flag("largePageTitles"),
            center_page_titles: flag("centerPageTitles"),
            export_title: texture("exportTitle"),
        })
    }
}

/// A whole book: its id, how it looks and its sections in order. Pages are addressed by one flat index
/// across the sections, as a reader flipping through meets them.
#[derive(Clone, Debug, PartialEq)]
pub struct Book {
    /// The book's id, such as `tconstruct:materials_and_you`.
    pub id: Identifier,
    /// How the book looks.
    pub appearance: BookAppearance,
    /// The book's sections, in order.
    pub sections: Vec<BookSection>,
}

impl Book {
    /// A book with the default appearance.
    pub fn new(id: Identifier, sections: Vec<BookSection>) -> Self {
        Self { id, appearance: BookAppearance::default(), sections }
    }

    /// How many pages the book has, across all sections.
    pub fn page_count(&self) -> usize {
        self.sections.iter().map(|section| section.pages.len()).sum()
    }

    /// The (section, page in section) a flat page index names; None past the end.
    pub fn locate(&self, mut page: usize) -> Option<(usize, usize)> {
        for (section, pages) in self.sections.iter().map(|s| s.pages.len()).enumerate() {
            if page < pages {
                return Some((section, page));
            }
            page -= pages;
        }
        None
    }

    /// The flat page index of a (section, page in section); None if either is out of range.
    pub fn flat_index(&self, section: usize, page: usize) -> Option<usize> {
        let mut flat = 0;
        for (index, current) in self.sections.iter().enumerate() {
            if index == section {
                return (page < current.pages.len()).then_some(flat + page);
            }
            flat += current.pages.len();
        }
        None
    }

    /// The page at a flat index.
    pub fn page(&self, page: usize) -> Option<&BookPage> {
        let (section, within) = self.locate(page)?;
        self.sections[section].pages.get(within)
    }
}

#[derive(Default)]
struct Registry {
    books: BTreeMap<String, Book>,
}

impl Registry {
    /// The shape every registered book must have, shared by registration and replacement.
    fn validate(book: &Book) -> Result<(), String> {
        if book.sections.is_empty() {
            return Err(format!("book {} has no sections", book.id));
        }
        for section in &book.sections {
            if section.name.is_empty() {
                return Err(format!("book {} has a section with no name", book.id));
            }
            if section.pages.is_empty() {
                return Err(format!("book {}'s section {} has no pages", book.id, section.name));
            }
        }
        Ok(())
    }

    fn register(&mut self, book: Book) -> Result<(), String> {
        Self::validate(&book)?;
        let key = book.id.to_string();
        if self.books.contains_key(&key) {
            return Err(format!("book {key} already registered"));
        }
        self.books.insert(key, book);
        Ok(())
    }

    fn replace(&mut self, book: Book) -> Result<(), String> {
        Self::validate(&book)?;
        self.books.insert(book.id.to_string(), book);
        Ok(())
    }

    fn get(&self, id: &Identifier) -> Option<Book> {
        self.books.get(&id.to_string()).cloned()
    }

    fn all(&self) -> Vec<Book> {
        self.books.values().cloned().collect()
    }
}

static BOOKS: Mutex<Registry> = Mutex::new(Registry { books: BTreeMap::new() });

/// The registry of books a reader can open. Registration happens once, while mods load; a book is
/// looked up whole when it opens, so what the reader sees never changes mid-read.
pub struct BookRegistry;

impl BookRegistry {
    /// Registers a book. Fails, keeping the earlier registration, for a duplicate id, a book with no
    /// sections, a section with no name or a section with no pages.
    pub fn register(book: Book) -> Result<(), String> {
        BOOKS.lock().map_err(|_| "book registry poisoned")?.register(book)
    }

    /// Registers a book, replacing any earlier registration of the same id (or registering it fresh when
    /// there is none): a mod whose book content is generated from its registries rebuilds the book when
    /// those registries reload, and the rebuild takes the first build's place. Fails, keeping the earlier
    /// registration, for a book with no sections, a section with no name or a section with no pages. A
    /// book already open keeps the copy it opened with, so a reader's pages never change mid-read.
    pub fn replace(book: Book) -> Result<(), String> {
        BOOKS.lock().map_err(|_| "book registry poisoned")?.replace(book)
    }

    /// The book registered under `id`, if any.
    pub fn get(id: &Identifier) -> Option<Book> {
        BOOKS.lock().ok()?.get(id)
    }

    /// Every registered book, by id.
    pub fn all() -> Vec<Book> {
        BOOKS.lock().map(|registry| registry.all()).unwrap_or_default()
    }
}

/// Where a reader is in a book: the book and the flat index of the page showing. The host keeps one
/// while the book screen is open; the turning rules are here so they can be tested without the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BookView {
    /// The book being read.
    pub book: Identifier,
    /// The flat index of the page showing.
    pub page: usize,
}

impl BookView {
    /// A view of `book` at its first page.
    pub fn new(book: Identifier) -> Self {
        Self { book, page: 0 }
    }

    /// Turns to the next page; false when already on the last.
    pub fn next_page(&mut self, book: &Book) -> bool {
        if self.page + 1 < book.page_count() {
            self.page += 1;
            true
        } else {
            false
        }
    }

    /// Turns to the previous page; false when already on the first.
    pub fn prev_page(&mut self, book: &Book) -> bool {
        if self.page > 0 {
            self.page -= 1;
            true
        } else {
            false
        }
    }

    /// Jumps to a section's first page; false for a section the book does not have.
    pub fn goto_section(&mut self, book: &Book, section: usize) -> bool {
        match book.flat_index(section, 0) {
            Some(page) => {
                self.page = page;
                true
            }
            None => false,
        }
    }

    /// The section the current page is in.
    pub fn section(&self, book: &Book) -> usize {
        book.locate(self.page).map_or(0, |(section, _)| section)
    }
}

/// What a click on the book screen means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookClick {
    /// Nothing was clicked.
    None,
    /// The next-page arrow.
    NextPage,
    /// The previous-page arrow.
    PrevPage,
    /// A section's row in the section list.
    Section(usize),
    /// A table row's link: the flat index of the page it opens.
    Page(usize),
}

/// The link rows of the page most recently drawn: the book and flat page they belong to, and each link
/// row's rect with the flat page it opens. [`draw_book`] records them and [`book_click`] reads them, so
/// a click is read against exactly the layout that was drawn — a table page's text runs wrap, so where a
/// row sits cannot be recomputed without the canvas that measured the text.
static LINK_ROWS: Mutex<Option<(Identifier, usize, Vec<([f32; 4], usize)>)>> = Mutex::new(None);

/// The renderer's geometry, in GUI pixels: the book panel is centered in the screen area and capped at
/// Mantle-like proportions; inside it, a section rail on the left (when the book has more than one
/// section) and the page on the right. [`draw_book`] and [`book_click`] share this one layout, so what
/// is drawn is exactly what can be clicked.
struct Layout {
    panel: [f32; 4],
    rail: Option<[f32; 4]>,
    content: [f32; 4],
    prev_arrow: [f32; 4],
    next_arrow: [f32; 4],
}

/// The panel's largest size in GUI pixels.
const PANEL_MAX: [f32; 2] = [340.0, 248.0];
/// The section rail's width when it shows.
const RAIL_WIDTH: f32 = 92.0;
/// Padding inside the panel.
const PAD: f32 = 10.0;
/// One line of text's height (the game's font is 9 tall; one of leading).
const LINE: f32 = 10.0;
/// A section row's height in the rail.
const ROW: f32 = 12.0;
/// A table row's height: the row's icon is 10 tall, with one of leading above and below.
const TABLE_ROW: f32 = 12.0;

fn layout(area: [f32; 4], book: &Book) -> Layout {
    let width = area[2].min(PANEL_MAX[0]).max(0.0);
    let height = area[3].min(PANEL_MAX[1]).max(0.0);
    let panel = [area[0] + (area[2] - width) / 2.0, area[1] + (area[3] - height) / 2.0, width, height];
    let rail = (book.sections.len() > 1).then(|| [panel[0] + PAD, panel[1] + PAD, RAIL_WIDTH, panel[3] - 2.0 * PAD]);
    let content_x = rail.map_or(panel[0] + PAD, |rail| rail[0] + rail[2] + PAD);
    let content = [content_x, panel[1] + PAD, panel[0] + panel[2] - PAD - content_x, panel[3] - 2.0 * PAD];
    let arrow_y = panel[1] + panel[3] - PAD - 12.0;
    Layout {
        panel,
        rail,
        content,
        prev_arrow: [content[0], arrow_y, 14.0, 12.0],
        next_arrow: [content[0] + content[2] - 14.0, arrow_y, 14.0, 12.0],
    }
}

fn inside(rect: [f32; 4], at: [f32; 2]) -> bool {
    at[0] >= rect[0] && at[0] < rect[0] + rect[2] && at[1] >= rect[1] && at[1] < rect[1] + rect[3]
}

/// What a click at `at` (GUI pixels, in the same space as `area`) means for the book in `view`.
pub fn book_click(book: &Book, view: &BookView, area: [f32; 4], at: [f32; 2]) -> BookClick {
    let layout = layout(area, book);
    if view.page + 1 < book.page_count() && inside(layout.next_arrow, at) {
        return BookClick::NextPage;
    }
    if view.page > 0 && inside(layout.prev_arrow, at) {
        return BookClick::PrevPage;
    }
    if let Some(rail) = layout.rail {
        if inside(rail, at) {
            let row = ((at[1] - rail[1]) / ROW) as usize;
            if row < book.sections.len() {
                return BookClick::Section(row);
            }
        }
    }
    if let Ok(rows) = LINK_ROWS.lock() {
        if let Some((book_id, page, rows)) = &*rows {
            if *book_id == book.id && *page == view.page {
                for (rect, target) in rows {
                    if inside(*rect, at) {
                        return BookClick::Page(*target);
                    }
                }
            }
        }
    }
    BookClick::None
}

/// The panel's background colour: the cover colour when the book sets one, else the renderer's parchment.
const PARCHMENT: u32 = 0xFFEF_E0BC;
/// The colour text is drawn in.
const INK: u32 = 0xFF2E_2015;
/// A dimmer ink for captions and unselected sections.
const FADED_INK: u32 = 0xFF6B_5636;
/// The colour behind a grid cell's item.
const CELL: u32 = 0xFFC9_B586;
/// The colour a table row's link is drawn in.
const LINK_INK: u32 = 0xFF2F_4E9E;

/// The stack a row's icon draws as: the icon's item, with its data as the stack's custom data.
fn icon_slot(icon: &BookIcon) -> ItemSlot {
    match icon {
        BookIcon::Item(item) => ItemSlot::new(item.clone(), 1),
        BookIcon::ItemWithData { item, data } => {
            let mut slot = ItemSlot::new(item.clone(), 1);
            slot.custom_data = data.clone();
            slot
        }
    }
}

/// Wraps `text` to lines of at most `width` GUI pixels, measured by the canvas. A word longer than a
/// line is split by characters; an empty text gives no lines.
pub fn wrap_text(canvas: &dyn ScreenCanvas, text: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
        if canvas.text_width(&candidate) <= width || line.is_empty() && canvas.text_width(word) <= width {
            line = candidate;
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
        }
        // The word alone may still be too wide: split it by characters.
        let mut rest = word;
        while canvas.text_width(rest) > width && !rest.is_empty() {
            let mut cut = rest.len();
            while cut > 0 && canvas.text_width(&rest[..cut]) > width {
                cut = rest.floor_char_boundary(cut - 1);
            }
            if cut == 0 {
                break;
            }
            lines.push(rest[..cut].to_owned());
            rest = &rest[cut..];
        }
        line = rest.to_owned();
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Draws the paragraphs of a text block starting at `y`, one wrapped line after another, a `paragraph`
/// run starting after a blank line; returns the y just past the last line. Drawing stops at `bottom`.
fn draw_text_block(canvas: &mut dyn ScreenCanvas, text: &[BookText], x: f32, mut y: f32, width: f32, bottom: f32, color: u32) -> f32 {
    for run in text {
        if run.paragraph && y > bottom - LINE {
            break;
        }
        if run.paragraph {
            y += LINE;
        }
        for line in wrap_text(canvas, &run.text, width) {
            if y > bottom - LINE {
                return y;
            }
            canvas.text(&line, [x, y], color, false);
            y += LINE;
        }
    }
    y
}

/// Draws the page `view` shows of `book` into `area` (the mod screen's panel rect, GUI pixels): the
/// panel, the section rail, the page's title and content, and the page arrows. The canvas's one text
/// size means `large_page_titles` cannot scale a title, and a crafting page draws its grid and recipe
/// id but not the recipe's items (resolving a recipe to its items is the consumer's side of the
/// screen); both are renderer limits, not data limits — the book keeps everything. A table page's
/// rows draw their icons as items and their links in the link colour, and the rows' places are
/// recorded for [`book_click`].
pub fn draw_book(canvas: &mut dyn ScreenCanvas, book: &Book, view: &BookView, area: [f32; 4]) {
    let layout = layout(area, book);
    let appearance = &book.appearance;
    // The page texture, when the book has one and the canvas knows it; otherwise the flat panel.
    let textured = appearance
        .book_texture
        .as_ref()
        .is_some_and(|texture| canvas.sprite(texture, [0.0, 0.0, 256.0, 256.0], layout.panel));
    if !textured {
        let fill = if appearance.cover_color != 0 { appearance.cover_color } else { PARCHMENT };
        canvas.fill(layout.panel, fill);
    }
    if let Some(rail) = layout.rail {
        let current = view.section(book);
        for (index, section) in book.sections.iter().enumerate() {
            let y = rail[1] + index as f32 * ROW;
            if y + ROW > rail[1] + rail[3] {
                break;
            }
            if index == current {
                canvas.fill([rail[0] - 2.0, y - 1.0, rail[2] + 4.0, ROW], CELL);
            }
            // The rail names its sections; the sections' icons are book data the rail does not draw.
            let label = section.name.as_str();
            let color = if index == current { INK } else { FADED_INK };
            canvas.text(label, [rail[0], y], color, false);
        }
    }
    let content = layout.content;
    let bottom = content[1] + content[3] - 16.0;
    let mut y = content[1];
    let Some(page) = book.page(view.page) else { return };
    if let Some(title) = page.title() {
        let x = if appearance.center_page_titles {
            content[0] + ((content[2] - canvas.text_width(title)) / 2.0).max(0.0)
        } else {
            content[0]
        };
        canvas.text(title, [x, y], INK, false);
        y += LINE + 4.0;
    }
    match page {
        BookPage::Text { text, .. } => {
            draw_text_block(canvas, text, content[0], y, content[2], bottom, INK);
        }
        BookPage::ImageText { image, text, .. } => {
            if image.width > 0 && image.height > 0 {
                let width = content[2].min(image.width as f32);
                let height = (width * image.height as f32 / image.width as f32).min((bottom - y) * 0.5);
                if height > 0.0 {
                    let dst = [content[0] + (content[2] - width) / 2.0, y, width, height];
                    if canvas.sprite(&image.file, [0.0, 0.0, image.width as f32, image.height as f32], dst) {
                        y += height + 4.0;
                    }
                }
            }
            draw_text_block(canvas, text, content[0], y, content[2], bottom, INK);
        }
        BookPage::Crafting { recipe, grid_size, description, .. } => {
            let cell = grid_size.cell();
            let slot = if appearance.slot_color != 0 { appearance.slot_color } else { CELL };
            for row in 0..3 {
                for col in 0..3 {
                    let x = content[0] + col as f32 * (cell + 2.0);
                    let yy = y + row as f32 * (cell + 2.0);
                    canvas.fill([x, yy, cell, cell], slot);
                    canvas.fill([x + 1.0, yy + 1.0, cell - 2.0, cell - 2.0], PARCHMENT);
                }
            }
            y += 3.0 * (cell + 2.0) + 2.0;
            canvas.text(&recipe.to_string(), [content[0], y], FADED_INK, false);
            y += LINE + 2.0;
            draw_text_block(canvas, description, content[0], y, content[2], bottom, INK);
        }
        BookPage::Table { intro, rows, text, .. } => {
            y = draw_text_block(canvas, intro, content[0], y, content[2], bottom, INK);
            let mut links = Vec::new();
            for row in rows {
                if y + TABLE_ROW > bottom + 4.0 {
                    break;
                }
                let (icon, label, value, link, header) = match row {
                    BookRow::Header { icon, text } => (icon, text.as_str(), None, None, true),
                    BookRow::Entry { icon, label, value, link } => (icon, label.as_str(), value.as_deref(), *link, false),
                };
                let mut x = content[0];
                if let Some(icon) = icon {
                    canvas.item(&icon_slot(icon), [x, y + 1.0], 10.0);
                    x += 14.0;
                }
                let color = if link.is_some() { LINK_INK } else { INK };
                canvas.text(label, [x, y + 1.0], color, false);
                if header {
                    let width = canvas.text_width(label);
                    canvas.fill([x, y + TABLE_ROW - 1.5, width, 1.0], FADED_INK);
                }
                if let Some(value) = value {
                    let value_x = content[0] + content[2] - canvas.text_width(value);
                    canvas.text(value, [value_x.max(x), y + 1.0], INK, false);
                }
                if let Some(target) = link {
                    links.push(([content[0], y, content[2], TABLE_ROW], target));
                }
                y += TABLE_ROW;
            }
            if let Ok(mut recorded) = LINK_ROWS.lock() {
                *recorded = Some((book.id.clone(), view.page, links));
            }
            draw_text_block(canvas, text, content[0], y + 2.0, content[2], bottom, INK);
        }
        BookPage::Opaque { kind, .. } => {
            canvas.text(kind, [content[0], y], FADED_INK, false);
        }
    }
    let arrow_color = if appearance.arrow_color != 0 { appearance.arrow_color } else { INK };
    if view.page > 0 {
        canvas.text("<", [layout.prev_arrow[0] + 3.0, layout.prev_arrow[1]], arrow_color, false);
    }
    if view.page + 1 < book.page_count() {
        canvas.text(">", [layout.next_arrow[0] + 3.0, layout.next_arrow[1]], arrow_color, false);
    }
}

/// What the game can do for a book reader. PlutoLib implements it over Horizon's mod screen; a test can
/// implement it (or use [`MemoryBookOps`]). The methods run on the client when a book opens: a host that
/// is not on the client answers false and nothing opens.
pub trait BookOps {
    /// Opens the book `book` for the local reader; false when the book is not registered or the screen
    /// cannot open now (another screen is up, the world is loading, the call is not on the client).
    fn open_book(&mut self, book: &Identifier) -> bool;
    /// Closes the book this mod opened; false when no book of this mod's is open.
    fn close_book(&mut self) -> bool;
    /// The book currently open through this reach, if any.
    fn open_book_id(&mut self) -> Option<Identifier>;
}

thread_local! {
    static OPS: Cell<Option<*mut dyn BookOps>> = const { Cell::new(None) };
}

struct Scope<'a> {
    previous: Option<*mut dyn BookOps>,
    _borrow: PhantomData<&'a mut dyn BookOps>,
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        OPS.with(|slot| slot.set(self.previous));
    }
}

/// Runs `f` with `ops` available to [`open_book`], [`close_book`] and [`current_book`] during `f`.
pub fn with_books<R>(ops: &mut dyn BookOps, f: impl FnOnce() -> R) -> R {
    // SAFETY: the pointer is only dereferenced while `f` runs, strictly inside the lifetime of `ops`; `Scope` clears it on every exit.
    let raw: *mut dyn BookOps = unsafe { std::mem::transmute::<&mut dyn BookOps, *mut dyn BookOps>(ops) };
    let previous = OPS.with(|slot| slot.replace(Some(raw)));
    let _scope = Scope { previous, _borrow: PhantomData };
    f()
}

fn with_current<R>(default: R, f: impl FnOnce(&mut dyn BookOps) -> R) -> R {
    match OPS.with(Cell::get) {
        // SAFETY: see `with_books`: the pointer is valid for as long as it is stored.
        Some(raw) => f(unsafe { &mut *raw }),
        None => default,
    }
}

/// Opens the book `book` for the local reader (Mantle: `BookScreen` opened from a book item's use, on the
/// client). False outside a callback the host opened the book scope for, for an unregistered book, or
/// when the screen cannot open now.
pub fn open_book(book: &Identifier) -> bool {
    BookRegistry::get(book).is_some() && with_current(false, |ops| ops.open_book(book))
}

/// Closes the open book; false when none is open through this reach.
pub fn close_book() -> bool {
    with_current(false, |ops| ops.close_book())
}

/// The book currently open through this reach, if any.
pub fn current_book() -> Option<Identifier> {
    with_current(None, |ops| ops.open_book_id())
}

/// An in-memory book reach for tests: opening records the book, closing clears it.
#[derive(Default, Debug)]
pub struct MemoryBookOps {
    /// The book currently open, if any.
    pub open: Option<Identifier>,
    /// Every book opened, in order.
    pub opens: Vec<Identifier>,
    /// How many times a book was closed.
    pub closes: u32,
}

impl BookOps for MemoryBookOps {
    fn open_book(&mut self, book: &Identifier) -> bool {
        self.open = Some(book.clone());
        self.opens.push(book.clone());
        true
    }

    fn close_book(&mut self) -> bool {
        if self.open.take().is_some() {
            self.closes += 1;
            true
        } else {
            false
        }
    }

    fn open_book_id(&mut self) -> Option<Identifier> {
        self.open.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder {
        fills: Vec<([f32; 4], u32)>,
        texts: Vec<(String, [f32; 2])>,
        sprites: Vec<String>,
    }

    impl ScreenCanvas for Recorder {
        fn fill(&mut self, rect: [f32; 4], argb: u32) {
            self.fills.push((rect, argb));
        }
        fn sprite(&mut self, name: &str, _: [f32; 4], _: [f32; 4]) -> bool {
            self.sprites.push(name.to_owned());
            true
        }
        fn text(&mut self, text: &str, at: [f32; 2], _: u32, _: bool) {
            self.texts.push((text.to_owned(), at));
        }
        fn text_width(&self, text: &str) -> f32 {
            text.len() as f32 * 6.0
        }
    }

    fn text_page(title: &str, body: &str) -> BookPage {
        BookPage::Text { title: Some(title.to_owned()), text: vec![BookText::plain(body)] }
    }

    fn sample_book() -> Book {
        Book::new(
            Identifier::parse("test:field_guide").unwrap(),
            vec![
                BookSection { name: "Start".into(), icon: None, pages: vec![text_page("Welcome", "hello world"), text_page("Second", "more text")] },
                BookSection {
                    name: "Recipes".into(),
                    icon: Some(BookIcon::Item(Identifier::parse("minecraft:crafting_table").unwrap())),
                    pages: vec![BookPage::Crafting {
                        title: Some("Table".into()),
                        recipe: Identifier::parse("minecraft:crafting_table").unwrap(),
                        grid_size: GridSize::Small,
                        description: vec![BookText::plain("craft things")],
                    }],
                },
            ],
        )
    }

    #[test]
    fn registry_rejects_duplicates_and_empty_books_and_keeps_the_first() {
        let mut registry = Registry::default();
        registry.register(sample_book()).unwrap();
        assert!(registry.register(sample_book()).is_err(), "duplicate id");
        let mut empty = sample_book();
        empty.id = Identifier::parse("test:empty").unwrap();
        empty.sections.clear();
        assert!(registry.register(empty).is_err(), "no sections");
        let mut no_pages = sample_book();
        no_pages.id = Identifier::parse("test:no_pages").unwrap();
        no_pages.sections[0].pages.clear();
        assert!(registry.register(no_pages).is_err(), "a section with no pages");
        assert_eq!(registry.get(&Identifier::parse("test:field_guide").unwrap()).unwrap().page_count(), 3);
        assert_eq!(registry.all().len(), 1);
    }

    #[test]
    fn flat_pages_walk_sections_in_order_and_views_clamp_at_the_ends() {
        let book = sample_book();
        assert_eq!(book.page_count(), 3);
        assert_eq!(book.locate(0), Some((0, 0)));
        assert_eq!(book.locate(2), Some((1, 0)));
        assert_eq!(book.locate(3), None);
        assert_eq!(book.flat_index(1, 0), Some(2));
        assert_eq!(book.flat_index(2, 0), None);
        let mut view = BookView::new(book.id.clone());
        assert!(!view.prev_page(&book));
        assert!(view.next_page(&book));
        assert!(view.next_page(&book));
        assert!(!view.next_page(&book), "the last page does not turn");
        assert_eq!(view.section(&book), 1);
        assert!(view.goto_section(&book, 0));
        assert_eq!(view.page, 0);
        assert!(!view.goto_section(&book, 5));
    }

    #[test]
    fn pages_decode_from_the_original_json_shapes_and_unknown_kinds_stay_opaque() {
        let text = serde_json::json!({"title": "Intro", "text": [{"text": "line one"}, {"text": "line two", "paragraph": true}]});
        let page = BookPage::from_json("mantle:text", &text).unwrap();
        assert_eq!(page.title(), Some("Intro"));
        match &page {
            BookPage::Text { text, .. } => {
                assert_eq!(text.len(), 2);
                assert!(!text[0].paragraph && text[1].paragraph);
            }
            other => panic!("expected a text page, got {other:?}"),
        }
        let crafting = serde_json::json!({"recipe": "tconstruct:smeltery/melting/iron", "grid_size": "large", "description": [{"text": "melt it"}]});
        match BookPage::from_json("mantle:crafting", &crafting).unwrap() {
            BookPage::Crafting { recipe, grid_size, .. } => {
                assert_eq!(recipe.to_string(), "tconstruct:smeltery/melting/iron");
                assert_eq!(grid_size, GridSize::Large);
            }
            other => panic!("expected a crafting page, got {other:?}"),
        }
        let smelting = serde_json::json!({"title": "Melting", "inputs": [1, 2]});
        match BookPage::from_json("mantle:smelting", &smelting).unwrap() {
            BookPage::Opaque { kind, data } => {
                assert_eq!(kind, "mantle:smelting");
                assert_eq!(data.get("title"), Some(&Nbt::String("Melting".into())));
            }
            other => panic!("expected an opaque page, got {other:?}"),
        }
        assert!(BookPage::from_json("mantle:crafting", &serde_json::json!({})).is_err(), "a crafting page without a recipe is an error, not an opaque page");
    }

    #[test]
    fn appearance_and_section_refs_decode_from_the_original_files() {
        let appearance = BookAppearance::from_json(&serde_json::json!({
            "coverTexture": "tconstruct:book/cover", "coverColor": -1, "drawSectionListText": true, "centerPageTitles": true
        }))
        .unwrap();
        assert_eq!(appearance.cover_texture.as_deref(), Some("tconstruct:book/cover"));
        assert_eq!(appearance.cover_color, 0xFFFF_FFFF, "Mantle writes colours as signed integers");
        assert!(appearance.draw_section_list_text && appearance.center_page_titles);
        assert_eq!(appearance.arrow_color, 0, "unset stays unset");
        let refs = section_page_refs(&serde_json::json!([
            {"name": "intro", "type": "mantle:text", "data": "intro"},
            {"name": "smeltery", "type": "mantle:crafting", "data": "smeltery"}
        ]))
        .unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[1].kind, "mantle:crafting");
        let icon = BookIcon::from_json(&serde_json::json!({"type": "forge:nbt", "item": "tconstruct:pickaxe", "nbt": {"tic_materials": "x"}})).unwrap();
        assert_eq!(icon.item().to_string(), "tconstruct:pickaxe");
        assert!(matches!(icon, BookIcon::ItemWithData { .. }));
    }

    #[test]
    fn wrapping_measures_with_the_canvas_and_splits_long_words() {
        let canvas = Recorder::default();
        assert_eq!(wrap_text(&canvas, "one two three four", 60.0), vec!["one two", "three four"]);
        assert_eq!(wrap_text(&canvas, "abcdefghijklm", 30.0), vec!["abcde", "fghij", "klm"]);
        assert!(wrap_text(&canvas, "", 30.0).is_empty());
    }

    #[test]
    fn drawing_and_clicking_share_one_layout() {
        let book = sample_book();
        let area = [0.0, 0.0, 400.0, 300.0];
        let mut view = BookView::new(book.id.clone());
        let mut canvas = Recorder::default();
        draw_book(&mut canvas, &book, &view, area);
        let drawn: Vec<&str> = canvas.texts.iter().map(|(text, _)| text.as_str()).collect();
        assert!(drawn.contains(&"Welcome") && drawn.contains(&"hello world"), "title and body are drawn: {drawn:?}");
        assert!(drawn.contains(&"Start") && drawn.contains(&"Recipes"), "the section rail is drawn");
        assert!(drawn.contains(&">") && !drawn.contains(&"<"), "only the next arrow on the first page");
        // The next arrow's own drawn position clicks as NextPage.
        let (_, at) = canvas.texts.iter().find(|(text, _)| text == ">").unwrap();
        assert_eq!(book_click(&book, &view, area, [at[0] + 1.0, at[1] + 1.0]), BookClick::NextPage);
        assert_eq!(book_click(&book, &view, area, [area[0] + 40.0, area[1] + 40.0]), BookClick::Section(0));
        assert_eq!(book_click(&book, &view, area, [area[0] + 200.0, area[1] + 150.0]), BookClick::None);
        // The crafting page draws its grid, its recipe id and its description.
        view.page = 2;
        let mut canvas = Recorder::default();
        draw_book(&mut canvas, &book, &view, area);
        assert_eq!(canvas.fills.len(), 1 + 1 + 18, "panel, current-section marker and 9 cells with inners");
        let drawn: Vec<&str> = canvas.texts.iter().map(|(text, _)| text.as_str()).collect();
        assert!(drawn.contains(&"minecraft:crafting_table") && drawn.contains(&"craft things"));
    }

    #[test]
    fn replace_swaps_a_registration_and_keeps_it_on_a_bad_book() {
        let mut registry = Registry::default();
        registry.register(sample_book()).unwrap();
        let id = Identifier::parse("test:field_guide").unwrap();
        let mut rebuilt = sample_book();
        rebuilt.sections.truncate(1);
        registry.replace(rebuilt).unwrap();
        assert_eq!(registry.get(&id).unwrap().page_count(), 2, "the rebuild took the first build's place");
        let mut bad = sample_book();
        bad.sections.clear();
        assert!(registry.replace(bad).is_err(), "a book with no sections is refused");
        assert_eq!(registry.get(&id).unwrap().page_count(), 2, "the earlier registration stands");
        let fresh = Book::new(Identifier::parse("test:fresh").unwrap(), sample_book().sections);
        registry.replace(fresh).unwrap();
        assert_eq!(registry.all().len(), 2, "replace also registers a book that was not there");
    }

    #[test]
    fn a_table_page_draws_its_rows_and_its_link_rows_click_through() {
        let icon = BookIcon::Item(Identifier::parse("minecraft:iron_ingot").unwrap());
        let book = Book::new(
            Identifier::parse("test:tables").unwrap(),
            vec![BookSection {
                name: "Materials".into(),
                icon: None,
                pages: vec![
                    BookPage::Table {
                        kind: "mantle:index".into(),
                        title: Some("Materials".into()),
                        intro: vec![BookText::plain("pick one")],
                        rows: vec![
                            BookRow::Header { icon: None, text: "Metals".into() },
                            BookRow::Entry { icon: Some(icon), label: "Iron".into(), value: Some("tier 2".into()), link: Some(1) },
                            BookRow::Entry { icon: None, label: "Stone".into(), value: None, link: None },
                        ],
                        text: vec![],
                    },
                    text_page("Iron", "the iron page"),
                ],
            }],
        );
        assert_eq!(book.page(0).map(|page| page.kind()), Some("mantle:index"), "a table page keeps its content's kind");
        let area = [0.0, 0.0, 400.0, 300.0];
        let view = BookView::new(book.id.clone());
        let mut canvas = Recorder::default();
        draw_book(&mut canvas, &book, &view, area);
        let drawn: Vec<&str> = canvas.texts.iter().map(|(text, _)| text.as_str()).collect();
        assert!(drawn.contains(&"Metals") && drawn.contains(&"Iron") && drawn.contains(&"tier 2") && drawn.contains(&"Stone"), "header, entries and values are drawn: {drawn:?}");
        // The Iron row's own drawn position clicks as a link to its page; the Stone row's does not.
        let (_, at) = canvas.texts.iter().find(|(text, _)| text == "Iron").unwrap();
        assert_eq!(book_click(&book, &view, area, [at[0] + 1.0, at[1] + 1.0]), BookClick::Page(1));
        let (_, at) = canvas.texts.iter().find(|(text, _)| text == "Stone").unwrap();
        assert_eq!(book_click(&book, &view, area, [at[0] + 1.0, at[1] + 1.0]), BookClick::None);
    }

    #[test]
    fn the_open_reach_answers_outside_a_scope_and_through_memory_ops() {
        let id = Identifier::parse("test:field_guide").unwrap();
        assert!(!open_book(&id), "no scope, and the book is not even registered");
        BookRegistry::register(sample_book()).unwrap();
        assert!(!open_book(&id), "registered, but outside a scope nothing opens");
        let mut ops = MemoryBookOps::default();
        with_books(&mut ops, || {
            assert!(open_book(&id));
            assert_eq!(current_book(), Some(id.clone()));
            assert!(close_book());
            assert_eq!(current_book(), None);
            assert!(!close_book(), "closing twice is false");
        });
        assert_eq!(ops.opens, vec![id]);
        assert_eq!(ops.closes, 1);
    }
}
