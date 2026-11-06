use crate::{font::Font, text::{ClickEvent, Span, Style, Text}};
use crate::consumer::{Book, Lines};
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(std::path::PathBuf, Font);
impl Fixture {
    fn new() -> Self { Self::provider(include_bytes!("guide_font_provider.json")) }
    fn provider(bytes: &[u8]) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!("tconstruct-guide-query-{}-{}",
            std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        let file = root.join("assets/minecraft/font/default.json");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, bytes).unwrap();
        let font = Font::metrics(&root);
        Self(root, font)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) { std::fs::remove_dir_all(&self.0).unwrap(); }
}

// Frozen pre-migration native traversal, source-materials-book-click.rs at
// 8715b1232c0fca133fe012f35830e2108e653d59. Not a modeled Font/Text.
fn old_click(font: &Font, text: &Text, width: f32, row: usize, mut x: f32) -> Option<ClickEvent> {
    let lines = font.split(text, width);
    let line = lines.get(row)?.resolve_legacy();
    for (c, style) in line.chars() {
        x -= font.advance(c, style);
        if x < 0.0 { return style.click.clone(); }
    }
    None
}
fn text(value: &str, target: i32) -> Text {
    Text::styled(value, Style { click: Some(ClickEvent::ChangePage(target)), ..Default::default() })
}
fn compare(font: &Font, pages: &[Text], width: f32, shared: bool) {
    let query = Book::new(pages);
    for (index, text) in pages.iter().enumerate() {
        let lines = query.page(font, index, width).unwrap();
        assert_eq!(matches!(lines, Lines::Shared(_)), shared);
        assert_eq!(lines.as_slice(), font.split(text, width));
        for row in [0, 1, 2, 16, 17, usize::MAX] {
            for x in [-f32::INFINITY, -1.0, -0.0, 0.0, 2.99, 3.0, 4.0, 7.99, 8.0,
                20.0, 182.0, f32::INFINITY, f32::NAN] {
                assert_eq!(lines.click(font, x, row), old_click(font, text, width, row, x),
                    "page={index} row={row} x={x}");
            }
        }
    }
    assert!(query.page(font, pages.len(), width).is_none());
}
#[test]
fn full_document_cross_page_links_use_shared_queries() {
    let f = Fixture::new();
    compare(&f.1, &[text("a😀\nb", 3), Text::literal(""), text("ab ab", 1)], 8.0, true);
}
#[test]
fn legacy_unicode_styles_and_zero_advances_keep_native_order() {
    let f = Fixture::new();
    let mut value = text("§la§rb zאב\n", 1);
    value.spans[0].style.bold = Some(false);
    value.spans[0].style.insertion = Some("arbitrary:opaque".into());
    value.spans.insert(0, Span { text: String::new(), style: Style { italic: Some(true), ..Default::default() } });
    for width in [1.0, 6.0, 8.0, 182.0] { compare(&f.1, &[value.clone()], width, true); }
}
#[test]
fn rejected_actions_and_native_styles_retain_exact_fallback() {
    let f = Fixture::new();
    for style in [
        Style { click: Some(ClickEvent::RunCommand("/unchanged".into())), ..Default::default() },
        Style { click: Some(ClickEvent::OpenUrl("https://example.invalid/".into())), ..Default::default() },
        Style { click: Some(ClickEvent::CopyToClipboard("unchanged".into())), ..Default::default() },
        Style { font: Some("minecraft:default".into()), click: Some(ClickEvent::ChangePage(1)), ..Default::default() },
        Style { shadow_color: Some(0xff00ff00), click: Some(ClickEvent::ChangePage(1)), ..Default::default() },
    ] { compare(&f.1, &[Text::styled("ab", style)], 8.0, false); }
}
#[test]
fn invalid_and_out_of_range_targets_are_not_deleted() {
    let f = Fixture::new();
    for target in [-1, 0, 2, i32::MAX] { compare(&f.1, &[text("ab", target)], 8.0, false); }
}
#[test]
fn unsupported_other_page_falls_back_without_renumbering() {
    let f = Fixture::new();
    let other = Text::styled("ab", Style { hover: Some(Box::new(Text::literal("hover"))), ..Default::default() });
    compare(&f.1, &[text("ab", 2), other], 8.0, false);
}
#[test]
fn document_and_work_budgets_retain_original_content() {
    let f = Fixture::new();
    let mut too_many = Text::empty();
    too_many.spans = vec![Span { text: "a".into(), style: Style::default() }; 33];
    compare(&f.1, &[too_many], 182.0, false);
    let mut metadata = text("a", 1);
    metadata.spans[0].style.insertion = Some("x".repeat(8193));
    compare(&f.1, &[metadata], 182.0, false);
    let mut amplification = text(&"a".repeat(130), 1);
    amplification.spans[0].style.insertion = Some("x".repeat(8192));
    compare(&f.1, &[amplification], 182.0, false);
    compare(&f.1, &[text(&"a".repeat(4097), 1)], 182.0, false);
}
#[test]
fn query_geometry_errors_use_native_fallback() {
    let f = Fixture::new();
    for width in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        compare(&f.1, &[text("ab", 1)], width, false);
    }
}
#[test]
fn cover_or_missing_page_never_converts_or_queries_font() {
    let f = Fixture::new();
    let pages = [text("a", 99)];
    let book = Book::new(&pages);
    // Same handler shape as a cover: construct the borrowed adapter but never
    // request a page. Invalid content must not cause mutation or an action.
    assert!(book.page(&f.1, 1, 182.0).is_none());
    assert_eq!(pages[0], text("a", 99));
    assert!(f.1.take_patches().is_empty());
}

#[test]
fn nonfinite_native_metric_falls_back_without_substitution() {
    let f = Fixture::provider(br#"{"providers":[{"type":"space","advances":{"a":1e100,"b":5}}]}"#);
    assert!(!f.1.advance('a', &Style::default()).is_finite());
    compare(&f.1, &[text("ab", 1)], 182.0, false);
}
#[test]
fn negative_native_metric_keeps_signed_advance_semantics() {
    let f = Fixture::provider(br#"{"providers":[{"type":"space","advances":{"a":-3,"b":5}}]}"#);
    assert_eq!(f.1.advance('a', &Style::default()), -3.0);
    compare(&f.1, &[text("ab", 1)], 182.0, true);
}
