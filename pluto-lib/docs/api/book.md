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

Three page kinds are decoded into their own variants — `mantle:text`, `mantle:image_text` and `mantle:crafting` — and drawn in full. Any other kind (a smelting page, a material page, ...) decodes to `BookPage::Opaque` with its JSON kept as NBT, so registering a book never loses a page; a renderer for the kind can be added without changing the book.

## Turning pages

Pages are addressed by one flat index across the sections, as a reader flipping through meets them. `BookView` holds where a reader is and applies the turning rules; the host keeps one while the book is open. On the book screen, the reader turns with the arrows at the page's foot, the mouse wheel, the arrow and page keys, and jumps with the section list on the left (drawn when the book has more than one section). Escape closes the book, as it closes any screen.

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

> **Use with Caution.** Host adapter written, not yet compiled or game-verified. The host opens the screen through the game's mod-screen call and draws through the screen hooks; two renderer limits are the canvas's, not the data's: the canvas has one text size, so `large_page_titles` is carried but cannot scale a title, and the canvas cannot draw items, so a crafting page draws its grid and recipe id but not the recipe's items.

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

A whole book: `id`, `appearance` and `sections` in order. `Book::new(id, sections)` gives it the default appearance. `page_count()` counts the pages across all sections, `locate(page)` turns a flat page index into `(section, page in section)`, `flat_index(section, page)` goes back, and `page(page)` borrows the page at a flat index.

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

One section of a book: its `name` for the section list, its optional `icon` and its `pages` in order.

### `BookPage`

One page. `Text` and `ImageText` carry a `title` and `text` runs (an image page also its `image`); `Crafting` carries a `title`, the `recipe` it shows, a `grid_size` and a `description`; `Opaque` keeps any other kind's `kind` id and `data`. `kind()` is the Mantle page kind's id and `title()` the page's title. `BookPage::from_json(kind, data)` decodes one localized page from its JSON (see [Building a book](#building-a-book-from-the-original-data)).

### `BookText`

One run of text on a page: `text` and `paragraph` (a blank line is drawn before a paragraph run). `BookText::plain` and `BookText::paragraph` build the two forms.

### `BookImage`

An image on a page: the GUI texture `file` and its `width` and `height` in texture pixels.

### `GridSize`

How big a crafting page draws its grid: `Small` (the default) or `Large`; `GridSize::parse` reads the book data's name.

### `BookIcon`

What a section's list entry shows: `Item(item)` or `ItemWithData { item, data }`. `BookIcon::from_json` decodes the index's `icon`; `item()` is the icon's item either way.

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

How a book looks: `cover_texture`, `cover_color`, `book_texture`, `arrow_color`, `slot_color` (a colour of 0 means the data did not set it), `draw_section_list_text`, `large_page_titles`, `center_page_titles` and `export_title`. `BookAppearance::from_json` decodes an `appearance.json`.

### `BookRegistry`

The registry of books a reader can open. `register(book)` fails, keeping the earlier registration, for a duplicate id, a book with no sections, a section with no name or a section with no pages. `get(id)` returns the registered book and `all()` every registered book, by id.

### `BookView`

Where a reader is in a book: the `book` and the flat `page` showing. `next_page` and `prev_page` turn one page and answer false at the ends; `goto_section` jumps to a section's first page; `section` says which section the current page is in.

### `BookClick` and `book_click`

What a click on the book screen means: `NextPage`, `PrevPage`, `Section(index)` or `None`. `book_click(book, view, area, at)` reads a click at `at` (GUI pixels, in the same space as `area`, the screen's panel rect) against the layout `draw_book` draws.

### `draw_book`

`fn draw_book(canvas, book, view, area)`. Draws the page `view` shows of `book` into `area`: the panel (the book's page texture when it has one and the canvas knows it, else a flat panel in the cover colour or the renderer's parchment), the section rail, the page's title and content, and the page arrows.

### `wrap_text`

`fn wrap_text(canvas, text, width) -> Vec<String>`. Wraps `text` to lines of at most `width` GUI pixels, measured by the canvas; a word longer than a line is split by characters.

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

What the game can do for a book reader: `open_book`, `close_book` and `open_book_id`. PlutoLib implements it over the game's mod screen; `MemoryBookOps` implements it in memory for tests, recording `opens`, the `open` book and `closes`.

### `with_books`, `open_book`, `close_book`, `current_book`

`with_books(ops, f)` runs `f` with a book reach available. Inside it, `open_book(id)` opens a registered book for the local reader (false for an unregistered book, when the screen cannot open now, or when the call is not on the client), `close_book()` closes the book this reach opened and `current_book()` names the book currently open through it. Outside a scope they answer false and `None`.
