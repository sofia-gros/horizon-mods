# Books

A book a reader opens in the game: sections of pages with text, pictures and crafting recipes, in the shape of the original Mantle book data. A mod registers the book once, and any callback can open it for the local reader; PlutoLib draws the pages and turns them. This page first walks through using it, then lists every item in the [Reference](#reference).

## Basic Example

A one-section field guide, registered at load and opened from a callback:

```rust
use pluto_lib::{Book, BookPage, BookRegistry, BookSection, BookText, Identifier};

let book = Book::new(
    Identifier::parse("example:field_guide").unwrap(),
    vec![BookSection {
        name: "Start".into(),
        icon: None,
        pages: vec![BookPage::Text {
            title: Some("Welcome".into()),
            text: vec![BookText::plain("Keep this book handy.")],
        }],
    }],
);
BookRegistry::register(book).unwrap();

use pluto_lib::{MemoryBookOps, open_book, with_books};
let mut ops = MemoryBookOps::default();
with_books(&mut ops, || {
    assert!(open_book(&Identifier::parse("example:field_guide").unwrap()));
});
assert_eq!(ops.opens.len(), 1);
```

`open_book` is a client act: the game opens PlutoLib's own screen over the world and the pages are drawn into it. A callback running on the server thread cannot open a client screen and gets `false`; a mod whose book item is used on the server tells the client over its own channel (see [networking](networking.md)) and the client side opens the book there.

## Building a book from the original data

The content model is the original book format. A book's folder holds an `index.json` (the sections, each with a `data` path and an `icon`), an `appearance.json`, one JSON per section listing its pages (`{name, type, data}`), and the pages themselves per language. The pieces decode from their JSON values; walking the files is the consumer's work (a JSON folder callback from [resources](resources.md) hands them over):

```rust
use pluto_lib::{BookAppearance, BookIcon, BookPage, section_page_refs};

let appearance = BookAppearance::from_json(&serde_json::json!({
    "coverTexture": "example:book/cover", "coverColor": -1, "drawSectionListText": true
})).unwrap();
assert_eq!(appearance.cover_color, 0xFFFF_FFFF, "colours are written as signed integers");
assert!(appearance.draw_section_list_text);

let refs = section_page_refs(&serde_json::json!([
    {"name": "intro", "type": "mantle:text", "data": "intro"}
])).unwrap();
assert_eq!(refs[0].kind, "mantle:text");

let page = BookPage::from_json("mantle:text", &serde_json::json!({
    "title": "Intro", "text": [{"text": "line one"}, {"text": "line two", "paragraph": true}]
})).unwrap();
assert_eq!(page.title(), Some("Intro"));

let icon = BookIcon::from_json(&serde_json::json!({"item": "minecraft:crafting_table"})).unwrap();
assert_eq!(icon.item().to_string(), "minecraft:crafting_table");
```

Three page kinds are decoded into their own variants — `mantle:text`, `mantle:image_text` and `mantle:crafting` — and drawn in full. Any other kind (a smelting page, ...) decodes to `BookPage::Opaque` with its JSON kept as NBT, so registering a book never loses a page; a renderer for the kind can be added without changing the book. Pages the original generates from its registries (a page per material, tool or modifier) are not in the files at all: the consumer builds them as `BookPage::Table` pages, keeping the generated content's own kind id, and rebuilds the book with `BookRegistry::replace` when its registries reload.

## Turning pages

Pages are addressed by one flat index across the sections, as a reader flipping through meets them. `BookView` holds where a reader is and applies the turning rules; the host keeps one while the book is open. On the book screen, the reader turns with the arrows at the page's foot, the mouse wheel, the arrow and page keys, jumps with the section list on the left (drawn when the book has more than one section), and follows a table row's link where a page has one. Escape closes the book, as it closes any screen.

```rust
use pluto_lib::{Book, BookPage, BookRegistry, BookSection, BookText, BookView, Identifier};

let book = Book::new(
    Identifier::parse("example:guide").unwrap(),
    vec![
        BookSection { name: "A".into(), icon: None, pages: vec![
            BookPage::Text { title: None, text: vec![BookText::plain("one")] },
            BookPage::Text { title: None, text: vec![BookText::plain("two")] },
        ]},
        BookSection { name: "B".into(), icon: None, pages: vec![
            BookPage::Text { title: None, text: vec![BookText::plain("three")] },
        ]},
    ],
);
assert_eq!(book.page_count(), 3);
assert_eq!(book.locate(2), Some((1, 0)));
let mut view = BookView::new(book.id.clone());
assert!(view.goto_section(&book, 1));
assert_eq!(view.page, 2);
assert!(!view.next_page(&book), "the last page does not turn");
BookRegistry::register(book).unwrap();
```

## Drawing

`draw_book` draws the current page onto a `ScreenCanvas` — the same surface the menu screens draw on (see [screen](screen.md)) — and `book_click` reads a click against the same layout, so what is drawn is exactly what can be clicked.

> **Use with Caution.** Host adapter written, not yet compiled or game-verified. The host opens the screen through the game's mod-screen call and draws through the screen hooks; two renderer limits are the canvas's, not the data's: the canvas has one text size, so `large_page_titles` is carried but cannot scale a title, and a crafting page draws its grid and recipe id but not the recipe's items (resolving a recipe to its items is the consumer's side of the screen). Table rows draw their icons as items.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, BookView, Identifier, ScreenCanvas, draw_book};

#[derive(Default)]
struct Canvas { texts: Vec<String> }
impl ScreenCanvas for Canvas {
    fn fill(&mut self, _: [f32; 4], _: u32) {}
    fn sprite(&mut self, _: &str, _: [f32; 4], _: [f32; 4]) -> bool { false }
    fn text(&mut self, text: &str, _: [f32; 2], _: u32, _: bool) { self.texts.push(text.to_owned()) }
    fn text_width(&self, text: &str) -> f32 { text.len() as f32 * 6.0 }
}

let book = Book::new(
    Identifier::parse("example:guide").unwrap(),
    vec![BookSection { name: "A".into(), icon: None, pages: vec![
        BookPage::Text { title: Some("Hi".into()), text: vec![BookText::plain("body")] },
        BookPage::Text { title: None, text: vec![BookText::plain("next")] },
        BookPage::Text { title: None, text: vec![BookText::plain("more")] },
    ]}],
);
let view = BookView::new(book.id.clone());
let mut canvas = Canvas::default();
draw_book(&mut canvas, &book, &view, [0.0, 0.0, 400.0, 300.0]);
assert!(canvas.texts.iter().any(|t| t == "Hi"));
assert!(canvas.texts.iter().any(|t| t == "body"));
assert!(canvas.texts.iter().any(|t| t == ">"), "the next-page arrow is drawn");
```

## Reference

### `Book`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A whole book: its `id`, `appearance` and `sections` in order.

> `Book::new(id, sections)` gives the book the default appearance.
> `page_count()` counts the pages across all sections, `locate(page)` turns a
> flat page index into `(section, page in section)`, `flat_index(section, page)`
> goes back, and `page(page)` borrows the page at a flat index.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, Identifier};

let book = Book::new(Identifier::parse("example:b").unwrap(), vec![
    BookSection { name: "S".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("x")] },
    ]},
]);
assert_eq!(book.flat_index(0, 0), Some(0));
assert_eq!(book.page(0).map(|p| p.kind()), Some("mantle:text"));
```

### `BookSection`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One section of a book.

> A section carries its `name` for the section list, its optional `icon`, and
> its `pages` in order. The sections are the units the book's section list jumps
> between, and their pages follow one another in the book's flat page order.

```rust
use pluto_lib::{BookPage, BookSection, BookText};

let section = BookSection {
    name: "Start".into(),
    icon: None,
    pages: vec![BookPage::Text { title: Some("Welcome".into()), text: vec![BookText::plain("hello")] }],
};
assert_eq!(section.name, "Start");
assert!(section.icon.is_none());
assert_eq!(section.pages.len(), 1);
```

### `BookPage`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One page of a book.

> `Text` and `ImageText` carry a `title` and `text` runs (an image page also its
> `image`); `TextImage` is the same pair with the text beside the image;
> `Showcase` carries a `title`, the `item` it shows large, its `tooltip` lines
> and a `text`; `Crafting` carries a `title`, the `recipe` it shows, a
> `grid_size`, the `ingredients` and `result` the grid draws (the page's JSON
> names only the recipe, so the mod that owns it fills those in) and a
> `description`; `Table` carries the `kind` of the content it stands for, a
> `title`, `intro` text, its `rows`, `text` below them and a `sidebar` of
> display items — the page a mod generates from its registries; `Structure`
> carries a `title`, the `data` id of its structure template and the `structure`
> the loader filled from it; `Opaque` keeps any other kind's `kind` id and
> `data`. `kind()` is the Mantle page kind's id and `title()` the page's title,
> and `BookPage::from_json(kind, data)` decodes one localized page from its
> JSON.

```rust
use pluto_lib::{BookPage, BookText};

let page = BookPage::Text {
    title: Some("Welcome".into()),
    text: vec![BookText::plain("hello world")],
};
assert_eq!(page.kind(), "mantle:text");
assert_eq!(page.title(), Some("Welcome"));
```

### `BookStructure`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A structure a book page shows: the blocks of a vanilla structure template as scene blocks, with the template's `size`.

> `BookStructure::from_template(template)` reads the template compound — the
> root of its binary NBT document, read with `Nbt::from_binary`. The palette's
> names and properties become each block's state, a block entry's own `nbt`
> becomes its scene entity's data, and air is left out. A structure page draws
> it turning slowly, one revolution a minute.

```rust
use pluto_lib::{BookStructure, Nbt, NbtCompound};

let mut entry = NbtCompound::new();
entry.insert("Name".into(), Nbt::String("minecraft:stone".into()));
let mut block = NbtCompound::new();
block.insert("pos".into(), Nbt::List(vec![Nbt::Int(0), Nbt::Int(0), Nbt::Int(0)]));
block.insert("state".into(), Nbt::Int(0));
let mut template = NbtCompound::new();
template.insert("size".into(), Nbt::List(vec![Nbt::Int(1), Nbt::Int(1), Nbt::Int(1)]));
template.insert("palette".into(), Nbt::List(vec![Nbt::Compound(entry)]));
template.insert("blocks".into(), Nbt::List(vec![Nbt::Compound(block)]));
let structure = BookStructure::from_template(&template).unwrap();
assert_eq!(structure.size, [1, 1, 1]);
assert_eq!(structure.blocks.len(), 1);
assert_eq!(structure.blocks[0].block.block.to_string(), "minecraft:stone");
```

### `BookStructure::from_template`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Reads a vanilla structure template compound into a structure.

> The compound is the root of the template's binary NBT document. The palette's
> names and properties become each block's state, a block entry's own `nbt`
> becomes its scene entity's data, and air is left out. An error names why the
> compound is not a template.

```rust
use pluto_lib::{BookStructure, NbtCompound};

assert!(BookStructure::from_template(&NbtCompound::new()).is_err());
```

### `BookRow`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One row of a table page.

> `Header { icon, text }` names what follows — a group, a stat block — and is
> drawn underlined. `Entry { icon, label, value, link }` is one entry of the
> table: its `label`, an optional `value` drawn right-aligned on the same line,
> an optional `icon`, and an optional `link` to the flat index of another page
> of the same book, which a click on the row opens. A row is one line tall when
> drawn; a table longer than a page is split into several table pages by its
> producer.

```rust
use pluto_lib::{BookIcon, BookRow, Identifier};

let header = BookRow::Header { icon: None, text: "Metals".into() };
assert!(matches!(header, BookRow::Header { .. }));
let entry = BookRow::Entry {
    icon: Some(BookIcon::Item(Identifier::parse("minecraft:iron_ingot").unwrap())),
    label: "Iron".into(),
    value: Some("tier 2".into()),
    link: Some(3),
};
assert!(matches!(entry, BookRow::Entry { link: Some(3), .. }));
```

### `BookText`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One run of text on a page.

> A run carries its `text` and a `paragraph` flag; a blank line is drawn before
> a paragraph run. `BookText::plain` and `BookText::paragraph` build the two
> forms.

```rust
use pluto_lib::BookText;

let plain = BookText::plain("line one");
assert_eq!(plain.text, "line one");
assert!(!plain.paragraph);
let paragraph = BookText::paragraph("line two");
assert!(paragraph.paragraph);
```

### `BookImage`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

An image on a page.

> The image names the GUI texture `file` and its `width` and `height` in texture
> pixels, which the page layout uses to place and size it.

```rust
use pluto_lib::BookImage;

let image = BookImage { file: "example:book/images/intro.png".into(), width: 100, height: 50 };
assert_eq!(image.file, "example:book/images/intro.png");
assert_eq!((image.width, image.height), (100, 50));
```

### `GridSize`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

How big a crafting page draws its grid.

> The sizes are `Small` (the default) and `Large`. `GridSize::parse` reads the
> book data's name into one of them.

```rust
use pluto_lib::GridSize;

assert_eq!(GridSize::parse("large"), GridSize::Large);
assert_eq!(GridSize::parse("small"), GridSize::Small);
assert_eq!(GridSize::default(), GridSize::Small);
```

### `BookIcon`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

What a section's list entry shows.

> Either `Item(item)` or `ItemWithData { item, data }`. `BookIcon::from_json`
> decodes the index's `icon`, and `item()` is the icon's item either way.

```rust
use pluto_lib::{BookIcon, Identifier};

let icon = BookIcon::Item(Identifier::parse("minecraft:crafting_table").unwrap());
assert_eq!(icon.item().to_string(), "minecraft:crafting_table");
```

### `SectionPageRef` and `section_page_refs`

A page entry of a section file before its localized page is loaded: `name`, `kind` and `data` (the language-relative path of the page's JSON). `section_page_refs(section_json)` reads a section file's entries.

```rust
use pluto_lib::section_page_refs;

let refs = section_page_refs(&serde_json::json!([
    {"name": "intro", "type": "mantle:text", "data": "intro"}
])).unwrap();
assert_eq!((refs[0].name.as_str(), refs[0].data.as_str()), ("intro", "intro"));
```

### `BookAppearance`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

How a book looks.

> The fields are `cover_texture`, `cover_color`, `book_texture`, `arrow_color`,
> `slot_color` (a colour of 0 means the data did not set it),
> `draw_section_list_text`, `large_page_titles`, `center_page_titles` and
> `export_title`. `BookAppearance::from_json` decodes an `appearance.json` into
> them.

```rust
use pluto_lib::BookAppearance;

let appearance = BookAppearance::from_json(&serde_json::json!({
    "coverTexture": "example:book/cover", "coverColor": -1, "drawSectionListText": true
})).unwrap();
assert_eq!(appearance.cover_texture.as_deref(), Some("example:book/cover"));
assert_eq!(appearance.cover_color, 0xFFFF_FFFF);
assert!(appearance.draw_section_list_text);
assert_eq!(appearance.arrow_color, 0);
```

### `BookRegistry`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The registry of books a reader can open.

> `register(book)` fails, keeping the earlier registration, for a duplicate id,
> a book with no sections, a section with no name or a section with no pages.
> `replace(book)` registers a book over any earlier registration of its id — the
> rebuild a mod makes when the registries its generated pages come from reload.
> `get(id)` returns the registered book and `all()` every registered book, by
> id.

```rust
use pluto_lib::{Book, BookPage, BookRegistry, BookSection, BookText, Identifier};

let id = Identifier::parse("example:registry_guide").unwrap();
let book = Book::new(id.clone(), vec![BookSection {
    name: "Start".into(), icon: None,
    pages: vec![BookPage::Text { title: None, text: vec![BookText::plain("x")] }],
}]);
BookRegistry::register(book).unwrap();
assert_eq!(BookRegistry::get(&id).unwrap().page_count(), 1);
assert!(BookRegistry::all().iter().any(|b| b.id == id));
```

### `BookView`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Where a reader is in a book: the `book` and the flat `page` showing.

> `next_page` and `prev_page` turn one spread and answer false at the ends.
> `goto_section` jumps to a section's first page, and `section` says which
> section the current page is in.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, BookView, Identifier};

let book = Book::new(Identifier::parse("example:view_guide").unwrap(), vec![
    BookSection { name: "A".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("one")] },
        BookPage::Text { title: None, text: vec![BookText::plain("two")] },
        BookPage::Text { title: None, text: vec![BookText::plain("three")] },
    ]},
]);
let mut view = BookView::new(book.id.clone());
assert_eq!(view.page, 0);
assert!(view.next_page(&book));
assert_eq!(view.page, 2);
assert!(!view.next_page(&book));
assert!(view.prev_page(&book));
assert_eq!(view.page, 0);
```

### `BookClick` and `book_click`

What a click on the book screen means: `NextPage`, `PrevPage`, `Section(index)`, `Page(index)` (a table row's link) or `None`. `book_click(book, view, area, at)` reads a click at `at` (GUI pixels, in the same space as `area`, the screen's panel rect) against the layout `draw_book` draws.

### `draw_book`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: inert](badges/sunrise-inert.svg)

Draws the page `view` shows of `book` into `area`.

`fn draw_book(canvas, book, view, area)`

> The drawing covers the panel — the book's page texture when it has one and the
> canvas knows it, else a flat panel in the cover colour or the renderer's
> parchment — the section rail, the page's title and content, and the page
> arrows.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, BookView, Identifier, ScreenCanvas, draw_book};

#[derive(Default)]
struct Canvas { texts: Vec<String> }
impl ScreenCanvas for Canvas {
    fn fill(&mut self, _: [f32; 4], _: u32) {}
    fn sprite(&mut self, _: &str, _: [f32; 4], _: [f32; 4]) -> bool { false }
    fn text(&mut self, text: &str, _: [f32; 2], _: u32, _: bool) { self.texts.push(text.to_owned()) }
    fn text_width(&self, text: &str) -> f32 { text.len() as f32 * 6.0 }
}

let book = Book::new(Identifier::parse("example:draw_guide").unwrap(), vec![
    BookSection { name: "A".into(), icon: None, pages: vec![
        BookPage::Text { title: Some("Hi".into()), text: vec![BookText::plain("body")] },
    ]},
]);
let view = BookView::new(book.id.clone());
let mut canvas = Canvas::default();
draw_book(&mut canvas, &book, &view, [0.0, 0.0, 400.0, 300.0]);
assert!(canvas.texts.iter().any(|t| t == "Hi"));
assert!(canvas.texts.iter().any(|t| t == "body"));
```

### `wrap_text`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Wraps `text` to lines of at most `width` GUI pixels.

`fn wrap_text(canvas, text, width) -> Vec<String>`

> Line widths are measured by the canvas, so the wrap matches what the canvas
> will draw. A word longer than a line is split by characters.

```rust
use pluto_lib::{ScreenCanvas, wrap_text};

struct Canvas;
impl ScreenCanvas for Canvas {
    fn fill(&mut self, _: [f32; 4], _: u32) {}
    fn sprite(&mut self, _: &str, _: [f32; 4], _: [f32; 4]) -> bool { false }
    fn text(&mut self, _: &str, _: [f32; 2], _: u32, _: bool) {}
    fn text_width(&self, text: &str) -> f32 { text.len() as f32 * 6.0 }
}
assert_eq!(wrap_text(&Canvas, "one two three four", 60.0), vec!["one two", "three four"]);
```

### `BookOps`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: inert](badges/sunrise-inert.svg)

What the game can do for a book reader: `open_book`, `close_book` and `open_book_id`.

> PlutoLib implements it over the game's mod screen. `MemoryBookOps` implements
> it in memory for tests, recording `opens`, the `open` book and `closes`.

```rust
use pluto_lib::{BookOps, Identifier, MemoryBookOps};

let id = Identifier::parse("example:ops_guide").unwrap();
let mut ops = MemoryBookOps::default();
assert!(ops.open_book(&id));
assert_eq!(ops.open_book_id(), Some(id.clone()));
assert!(ops.close_book());
assert_eq!(ops.open_book_id(), None);
```

### `with_books`, `open_book`, `close_book`, `current_book`

`with_books(ops, f)` runs `f` with a book reach available. Inside it, `open_book(id)` opens a registered book for the local reader (false for an unregistered book, when the screen cannot open now, or when the call is not on the client), `close_book()` closes the book this reach opened and `current_book()` names the book currently open through it. Outside a scope they answer false and `None`.

### `BookText::plain`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Builds a plain run of text.

> Built with `fn plain(text: &str) -> BookText`: the `text` is kept as given and
> `paragraph` is false, so no blank line is drawn before the run.

```rust
use pluto_lib::BookText;

let run = BookText::plain("line one");
assert_eq!(run.text, "line one");
assert!(!run.paragraph);
```

### `BookText::paragraph`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Builds a paragraph run of text.

> Built with `fn paragraph(text: &str) -> BookText`: the `text` is kept as given
> and `paragraph` is true, so a blank line is drawn before the run.

```rust
use pluto_lib::BookText;

let run = BookText::paragraph("line two");
assert_eq!(run.text, "line two");
assert!(run.paragraph);
```

### `GridSize::parse`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Reads the book data's size name.

> Called as `fn parse(name: &str) -> GridSize`: `"large"` gives `Large`, and any
> other name gives `Small`, so an unknown name falls back to the small grid.

```rust
use pluto_lib::GridSize;

assert_eq!(GridSize::parse("large"), GridSize::Large);
assert_eq!(GridSize::parse("small"), GridSize::Small);
assert_eq!(GridSize::parse("unknown"), GridSize::Small);
```

### `BookPage::kind`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The Mantle page kind's id.

> Read with `fn kind(&self) -> &str`: for example `mantle:text` for a `Text`
> page, or the stored kind for an `Opaque` page.

```rust
use pluto_lib::{BookPage, BookText};

let page = BookPage::Text { title: None, text: vec![BookText::plain("x")] };
assert_eq!(page.kind(), "mantle:text");
```

### `BookPage::title`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The page's title, if it has one.

> Read with `fn title(&self) -> Option<&str>`. An opaque page's title is read
> from its data's `title` string.

```rust
use pluto_lib::{BookPage, BookText};

let page = BookPage::Text { title: Some("Intro".into()), text: vec![BookText::plain("x")] };
assert_eq!(page.title(), Some("Intro"));
let untitled = BookPage::Text { title: None, text: vec![] };
assert_eq!(untitled.title(), None);
```

### `BookPage::from_json`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Decodes one localized page from its JSON, by its Mantle kind.

> Called as
> `fn from_json(kind: &str, data: &serde_json::Value) -> Result<BookPage, String>`.
> Unknown kinds decode to `Opaque` with the JSON kept as NBT, so registering a
> book never loses a page.

```rust
use pluto_lib::BookPage;

let page = BookPage::from_json("mantle:text", &serde_json::json!({
    "title": "Intro", "text": [{"text": "line one"}, {"text": "line two", "paragraph": true}]
})).unwrap();
assert_eq!(page.title(), Some("Intro"));
match &page {
    BookPage::Text { text, .. } => {
        assert_eq!(text.len(), 2);
        assert!(!text[0].paragraph && text[1].paragraph);
    }
    other => panic!("expected a text page, got {other:?}"),
}
assert!(BookPage::from_json("mantle:crafting", &serde_json::json!({})).is_err());
```

### `BookIcon::from_json`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Decodes a section's `icon` from the index JSON.

> Called as
> `fn from_json(value: &serde_json::Value) -> Result<BookIcon, String>`: an
> `nbt` field gives `ItemWithData`, otherwise the icon is `Item`.

```rust
use pluto_lib::BookIcon;

let icon = BookIcon::from_json(&serde_json::json!({
    "type": "forge:nbt", "item": "example:pickaxe", "nbt": {"tic_materials": "x"}
})).unwrap();
assert_eq!(icon.item().to_string(), "example:pickaxe");
assert!(matches!(icon, BookIcon::ItemWithData { .. }));
let plain = BookIcon::from_json(&serde_json::json!({"item": "minecraft:crafting_table"})).unwrap();
assert!(matches!(plain, BookIcon::Item(_)));
```

### `BookIcon::item`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The icon's item, either way.

> Read with `fn item(&self) -> &Identifier`. It answers the same for both `Item`
> and `ItemWithData`, so callers need not match on the variant first.

```rust
use pluto_lib::{BookIcon, Identifier};

let icon = BookIcon::Item(Identifier::parse("minecraft:crafting_table").unwrap());
assert_eq!(icon.item().to_string(), "minecraft:crafting_table");
```

### `SectionPageRef`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A page entry of a section file, before its localized page is loaded.

> It carries its `name` within the section, its `kind` — the page kind's id,
> such as `mantle:text` — and its `data`, the path of the page's localized JSON
> relative to the book's language folder.

```rust
use pluto_lib::SectionPageRef;

let entry = SectionPageRef { name: "intro".into(), kind: "mantle:text".into(), data: "intro".into() };
assert_eq!(entry.name, "intro");
assert_eq!(entry.kind, "mantle:text");
assert_eq!(entry.data, "intro");
```

### `section_page_refs`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Reads a section file's page entries.

> Called as
> `fn section_page_refs(section_json: &serde_json::Value) -> Result<Vec<SectionPageRef>, String>`.
> The consumer loads each entry's page JSON and decodes it with
> `BookPage::from_json`.

```rust
use pluto_lib::section_page_refs;

let refs = section_page_refs(&serde_json::json!([
    {"name": "intro", "type": "mantle:text", "data": "intro"},
    {"name": "smeltery", "type": "mantle:crafting", "data": "smeltery"}
])).unwrap();
assert_eq!(refs.len(), 2);
assert_eq!(refs[1].name, "smeltery");
assert_eq!(refs[1].kind, "mantle:crafting");
assert_eq!(refs[1].data, "smeltery");
assert!(section_page_refs(&serde_json::json!({})).is_err());
```

### `BookAppearance::from_json`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Decodes a book's `appearance.json`.

> Called as
> `fn from_json(data: &serde_json::Value) -> Result<BookAppearance, String>`.
> Colours are the JSON integers Mantle writes, and a missing field keeps its
> default.

```rust
use pluto_lib::BookAppearance;

let appearance = BookAppearance::from_json(&serde_json::json!({
    "coverTexture": "example:book/cover", "coverColor": -1,
    "drawSectionListText": true, "centerPageTitles": true
})).unwrap();
assert_eq!(appearance.cover_texture.as_deref(), Some("example:book/cover"));
assert_eq!(appearance.cover_color, 0xFFFF_FFFF);
assert!(appearance.draw_section_list_text && appearance.center_page_titles);
assert_eq!(appearance.arrow_color, 0);
```

### `Book::new`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A book with the default appearance.

> Built with `fn new(id: Identifier, sections: Vec<BookSection>) -> Book` from
> its `id` and its `sections` in order — the order the reader turns through
> them.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, Identifier};

let book = Book::new(Identifier::parse("example:new_book").unwrap(), vec![
    BookSection { name: "Start".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("x")] },
    ]},
]);
assert_eq!(book.id.to_string(), "example:new_book");
assert_eq!(book.sections.len(), 1);
assert_eq!(book.appearance.cover_color, 0);
```

### `Book::page_count`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

How many pages the book has across all sections.

> Read with `fn page_count(&self) -> usize`. The count spans every section,
> matching the flat page index the reader turns through.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, Identifier};

let book = Book::new(Identifier::parse("example:counted").unwrap(), vec![
    BookSection { name: "A".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("one")] },
        BookPage::Text { title: None, text: vec![BookText::plain("two")] },
    ]},
    BookSection { name: "B".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("three")] },
    ]},
]);
assert_eq!(book.page_count(), 3);
```

### `Book::locate`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The `(section, page in section)` a flat page index names.

> Read with `fn locate(&self, page: usize) -> Option<(usize, usize)>`; it
> answers `None` past the end. `flat_index` is the inverse lookup.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, Identifier};

let book = Book::new(Identifier::parse("example:located").unwrap(), vec![
    BookSection { name: "A".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("one")] },
        BookPage::Text { title: None, text: vec![BookText::plain("two")] },
    ]},
    BookSection { name: "B".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("three")] },
    ]},
]);
assert_eq!(book.locate(0), Some((0, 0)));
assert_eq!(book.locate(2), Some((1, 0)));
assert_eq!(book.locate(3), None);
```

### `Book::flat_index`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The flat page index of a `(section, page in section)`.

> Read with
> `fn flat_index(&self, section: usize, page: usize) -> Option<usize>`; it
> answers `None` if either the section or the page is out of range. `locate` is
> the inverse lookup.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, Identifier};

let book = Book::new(Identifier::parse("example:flat").unwrap(), vec![
    BookSection { name: "A".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("one")] },
        BookPage::Text { title: None, text: vec![BookText::plain("two")] },
    ]},
    BookSection { name: "B".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("three")] },
    ]},
]);
assert_eq!(book.flat_index(1, 0), Some(2));
assert_eq!(book.flat_index(0, 1), Some(1));
assert_eq!(book.flat_index(2, 0), None);
```

### `Book::page`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The page at a flat index.

> Read with `fn page(&self, page: usize) -> Option<&BookPage>`; it answers
> `None` past the end of the book.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, Identifier};

let book = Book::new(Identifier::parse("example:paged").unwrap(), vec![
    BookSection { name: "A".into(), icon: None, pages: vec![
        BookPage::Text { title: Some("First".into()), text: vec![BookText::plain("one")] },
    ]},
]);
assert_eq!(book.page(0).map(|p| p.title()), Some(Some("First")));
assert!(book.page(1).is_none());
```

### `BookSpread`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One two-page spread of a book.

> `left` and `right` are the flat page indices on its halves. A half is `None`
> when the spread ends the book or a padding marker leaves it blank; a half
> naming a `BookPage::Padding` page draws blank — a `mantle:blank` page that
> took the half.

```rust
use pluto_lib::book_api_v1::BookSpread;

let spread = BookSpread { left: Some(0), right: Some(1) };
assert_eq!((spread.left, spread.right), (Some(0), Some(1)));
```

### `BookSpread::contains`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Whether this spread is the one a reader of flat page `page` is looking at.

> Read with `fn contains(&self, page: usize) -> bool`. Use it to find the spread
> to draw for the view's current page.

```rust
use pluto_lib::book_api_v1::BookSpread;

let spread = BookSpread { left: Some(2), right: None };
assert!(spread.contains(2));
assert!(!spread.contains(3));
```

### `spreads`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Pairs a book's flat pages into the two-page spreads a reader turns through.

> Called as `fn spreads(book: &Book) -> Vec<BookSpread>`, consuming the book's
> padding markers as Mantle's book layout does: pages fill a spread's left half,
> then its right; a `mantle:blank` page takes whichever half comes next and
> draws blank there; a `mantle:right_padding` page must take a right half, so
> with no left half pending it is dropped. Spreads that would show nothing at
> all — padding alone — are left out.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, Identifier};
use pluto_lib::book_api_v1::{spreads, BookSpread};

let text = |title: &str| BookPage::Text { title: Some(title.into()), text: vec![BookText::plain(title)] };
let book = Book::new(Identifier::parse("example:spread").unwrap(), vec![
    BookSection { name: "One".into(), icon: None, pages: vec![text("a"), text("b"), text("c")] },
]);
assert_eq!(spreads(&book), vec![
    BookSpread { left: Some(0), right: Some(1) },
    BookSpread { left: Some(2), right: None },
]);
```

### `BookRegistry::register`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Registers a book.

> Called as `fn register(book: Book) -> Result<(), String>`. It fails, keeping
> the earlier registration, for a duplicate id, a book with no sections, a
> section with no name or a section with no pages.

```rust
use pluto_lib::{Book, BookPage, BookRegistry, BookSection, BookText, Identifier};

let book = Book::new(Identifier::parse("example:register_guide").unwrap(), vec![
    BookSection { name: "Start".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("x")] },
    ]},
]);
assert!(BookRegistry::register(book.clone()).is_ok());
assert!(BookRegistry::register(book).is_err(), "duplicate id");
```

### `BookRegistry::replace`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Registers a book over any earlier registration of the same id.

> Called as `fn replace(book: Book) -> Result<(), String>`. It fails, keeping
> the earlier registration, for a book with no sections, a section with no name
> or a section with no pages. A book already open keeps the copy it opened with.

```rust
use pluto_lib::{Book, BookPage, BookRegistry, BookSection, BookText, Identifier};

let id = Identifier::parse("example:replace_guide").unwrap();
let build = |body: &str| Book::new(id.clone(), vec![
    BookSection { name: "Start".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain(body)] },
    ]},
]);
BookRegistry::replace(build("first")).unwrap();
BookRegistry::replace(build("rebuilt")).unwrap();
assert_eq!(BookRegistry::get(&id).unwrap().page_count(), 1);
let mut bad = build("bad");
bad.sections.clear();
assert!(BookRegistry::replace(bad).is_err(), "a book with no sections is refused");
```

### `BookRegistry::get`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The book registered under an id, if any.

> Read with `fn get(id: &Identifier) -> Option<Book>`; it answers `None` when no
> book is registered under the id.

```rust
use pluto_lib::{Book, BookPage, BookRegistry, BookSection, BookText, Identifier};

let id = Identifier::parse("example:get_guide").unwrap();
BookRegistry::register(Book::new(id.clone(), vec![
    BookSection { name: "Start".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("x")] },
    ]},
])).unwrap();
assert_eq!(BookRegistry::get(&id).unwrap().page_count(), 1);
assert!(BookRegistry::get(&Identifier::parse("example:missing").unwrap()).is_none());
```

### `BookRegistry::all`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Every registered book, by id.

> Read with `fn all() -> Vec<Book>`. Use `get` instead when you already know the
> id you want.

```rust
use pluto_lib::{Book, BookPage, BookRegistry, BookSection, BookText, Identifier};

let id = Identifier::parse("example:all_guide").unwrap();
BookRegistry::register(Book::new(id.clone(), vec![
    BookSection { name: "Start".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("x")] },
    ]},
])).unwrap();
assert!(BookRegistry::all().iter().any(|b| b.id == id));
```

### `BookView::new`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A view of a book at its first page.

> Built with `fn new(book: Identifier) -> BookView`: its `book` is the given id
> and its `page` is 0.

```rust
use pluto_lib::{BookView, Identifier};

let view = BookView::new(Identifier::parse("example:viewed").unwrap());
assert_eq!(view.book.to_string(), "example:viewed");
assert_eq!(view.page, 0);
```

### `BookView::next_page`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Turns to the next spread.

> Called as `fn next_page(&mut self, book: &Book) -> bool`. It answers false
> when already on the last spread, and the `page` does not move then.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, BookView, Identifier};

let book = Book::new(Identifier::parse("example:next_guide").unwrap(), vec![
    BookSection { name: "A".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("one")] },
        BookPage::Text { title: None, text: vec![BookText::plain("two")] },
        BookPage::Text { title: None, text: vec![BookText::plain("three")] },
    ]},
]);
let mut view = BookView::new(book.id.clone());
assert!(view.next_page(&book));
assert_eq!(view.page, 2);
assert!(!view.next_page(&book));
assert_eq!(view.page, 2);
```

### `BookView::prev_page`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Turns to the previous spread.

> Called as `fn prev_page(&mut self, book: &Book) -> bool`. It answers false
> when already on the first spread, and the `page` does not move then.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, BookView, Identifier};

let book = Book::new(Identifier::parse("example:prev_guide").unwrap(), vec![
    BookSection { name: "A".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("one")] },
        BookPage::Text { title: None, text: vec![BookText::plain("two")] },
        BookPage::Text { title: None, text: vec![BookText::plain("three")] },
    ]},
]);
let mut view = BookView::new(book.id.clone());
assert!(!view.prev_page(&book));
assert!(view.next_page(&book));
assert!(view.prev_page(&book));
assert_eq!(view.page, 0);
```

### `BookView::goto_section`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Jumps to a section's first page.

> Called as `fn goto_section(&mut self, book: &Book, section: usize) -> bool`.
> It answers false for a section the book does not have.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, BookView, Identifier};

let book = Book::new(Identifier::parse("example:goto_guide").unwrap(), vec![
    BookSection { name: "A".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("one")] },
        BookPage::Text { title: None, text: vec![BookText::plain("two")] },
    ]},
    BookSection { name: "B".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("three")] },
    ]},
]);
let mut view = BookView::new(book.id.clone());
assert!(view.goto_section(&book, 1));
assert_eq!(view.page, 2);
assert!(!view.goto_section(&book, 5));
```

### `BookView::section`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The section the current page is in.

> Read with `fn section(&self, book: &Book) -> usize`. The answer follows the
> view's flat `page`, so it changes as the reader turns.

```rust
use pluto_lib::{Book, BookPage, BookSection, BookText, BookView, Identifier};

let book = Book::new(Identifier::parse("example:section_guide").unwrap(), vec![
    BookSection { name: "A".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("one")] },
    ]},
    BookSection { name: "B".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("two")] },
        BookPage::Text { title: None, text: vec![BookText::plain("three")] },
    ]},
]);
let mut view = BookView::new(book.id.clone());
assert_eq!(view.section(&book), 0);
assert!(view.next_page(&book));
assert_eq!(view.section(&book), 1);
```

### `BookClick`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

What a click on the book screen means.

> `None` when nothing was clicked, `NextPage` for the next-page arrow,
> `PrevPage` for the previous-page arrow, `Section(index)` for a section's row
> in the section list, or `Page(index)` for a table row's link — the flat index
> of the page it opens. A link click is read against the rows the last
> `draw_book` of that page drew, so it lands exactly on the row that was drawn.

```rust
use pluto_lib::BookClick;

assert_eq!(BookClick::None, BookClick::None);
assert_ne!(BookClick::NextPage, BookClick::PrevPage);
assert_eq!(BookClick::Section(1), BookClick::Section(1));
assert_eq!(BookClick::Page(4), BookClick::Page(4));
```

### `book_click`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

What a click at a position means.

> Called as
> `fn book_click(book: &Book, view: &BookView, area: [f32; 4], at: [f32; 2]) -> BookClick`.
> The click is read against the same layout `draw_book` draws, so what is drawn
> is exactly what can be clicked.

```rust
use pluto_lib::{Book, BookClick, BookPage, BookSection, BookText, BookView, Identifier, book_click};

let book = Book::new(Identifier::parse("example:click_guide").unwrap(), vec![
    BookSection { name: "A".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("one")] },
    ]},
    BookSection { name: "B".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("two")] },
    ]},
]);
let view = BookView::new(book.id.clone());
let area = [0.0, 0.0, 400.0, 300.0];
assert_eq!(book_click(&book, &view, area, [40.0, 40.0]), BookClick::Section(0));
assert_eq!(book_click(&book, &view, area, [200.0, 150.0]), BookClick::None);
```

### `with_books`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: inert](badges/sunrise-inert.svg)

Runs a closure with a book reach available.

> Called as `fn with_books(ops: &mut dyn BookOps, f: impl FnOnce() -> R) -> R`,
> returning the closure's result. Inside it, `open_book`, `close_book` and
> `current_book` use that reach.

```rust
use pluto_lib::{Book, BookPage, BookRegistry, BookSection, BookText, Identifier, MemoryBookOps, open_book, with_books};

let id = Identifier::parse("example:with_guide").unwrap();
BookRegistry::register(Book::new(id.clone(), vec![
    BookSection { name: "Start".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("x")] },
    ]},
])).unwrap();
let mut ops = MemoryBookOps::default();
let opened = with_books(&mut ops, || open_book(&id));
assert!(opened);
assert_eq!(ops.opens, vec![id]);
```

### `open_book`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: inert](badges/sunrise-inert.svg)

Opens a registered book for the local reader.

> Called as `fn open_book(book: &Identifier) -> bool`. It answers false outside
> a `with_books` scope, for an unregistered book, or when the screen cannot open
> now.

```rust
use pluto_lib::{Book, BookPage, BookRegistry, BookSection, BookText, Identifier, MemoryBookOps, open_book, with_books};

let id = Identifier::parse("example:open_guide").unwrap();
assert!(!open_book(&id), "outside a scope nothing opens");
BookRegistry::register(Book::new(id.clone(), vec![
    BookSection { name: "Start".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("x")] },
    ]},
])).unwrap();
let mut ops = MemoryBookOps::default();
with_books(&mut ops, || {
    assert!(open_book(&id));
});
assert_eq!(ops.opens, vec![id]);
```

### `close_book`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: inert](badges/sunrise-inert.svg)

Closes the book opened through the current reach.

> Called as `fn close_book() -> bool`. It answers false when no book is open
> through this reach.

```rust
use pluto_lib::{Book, BookPage, BookRegistry, BookSection, BookText, Identifier, MemoryBookOps, close_book, open_book, with_books};

let id = Identifier::parse("example:close_guide").unwrap();
BookRegistry::register(Book::new(id.clone(), vec![
    BookSection { name: "Start".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("x")] },
    ]},
])).unwrap();
let mut ops = MemoryBookOps::default();
with_books(&mut ops, || {
    assert!(open_book(&id));
    assert!(close_book());
    assert!(!close_book(), "closing twice is false");
});
assert_eq!(ops.closes, 1);
```

### `current_book`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: inert](badges/sunrise-inert.svg)

The book currently open through the current reach, if any.

> Read with `fn current_book() -> Option<Identifier>`. It answers `None` outside
> a `with_books` scope or when no book is open.

```rust
use pluto_lib::{Book, BookPage, BookRegistry, BookSection, BookText, Identifier, MemoryBookOps, close_book, current_book, open_book, with_books};

let id = Identifier::parse("example:current_guide").unwrap();
assert_eq!(current_book(), None);
BookRegistry::register(Book::new(id.clone(), vec![
    BookSection { name: "Start".into(), icon: None, pages: vec![
        BookPage::Text { title: None, text: vec![BookText::plain("x")] },
    ]},
])).unwrap();
let mut ops = MemoryBookOps::default();
with_books(&mut ops, || {
    assert!(open_book(&id));
    assert_eq!(current_book(), Some(id.clone()));
    assert!(close_book());
    assert_eq!(current_book(), None);
});
```

### `MemoryBookOps`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

An in-memory book reach for tests.

> Its `open` field is the book currently open, its `opens` field records every
> book opened in order, and its `closes` field counts how many times a book was
> closed. Opening records the book and closing clears `open`.

```rust
use pluto_lib::{BookOps, Identifier, MemoryBookOps};

let id = Identifier::parse("example:memory_guide").unwrap();
let mut ops = MemoryBookOps::default();
assert!(ops.open.is_none());
assert!(ops.open_book(&id));
assert_eq!(ops.open, Some(id.clone()));
assert_eq!(ops.opens, vec![id]);
assert!(ops.close_book());
assert!(ops.open.is_none());
assert_eq!(ops.closes, 1);
```
