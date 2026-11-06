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
fn nonfinite_native_metric_keeps_original_arithmetic_without_replay() {
    let f = Fixture::provider(br#"{"providers":[{"type":"space","advances":{"a":1e100,"b":5}}]}"#);
    assert!(!f.1.advance('a', &Style::default()).is_finite());
    compare(&f.1, &[text("ab", 1)], 182.0, true);
}
#[test]
fn negative_native_metric_keeps_signed_advance_semantics() {
    let f = Fixture::provider(br#"{"providers":[{"type":"space","advances":{"a":-3,"b":5}}]}"#);
    assert_eq!(f.1.advance('a', &Style::default()), -3.0);
    compare(&f.1, &[text("ab", 1)], 182.0, true);
}

// Original 16x8 RGBA test image: two 8-pixel cells, opaque widths 2 and 6.
// This is not a game asset or renderer. Font::load executes its real CPU
// rasterization/atlas-region allocator and exposes pending pixel patches.
const ORIGINAL_BITMAP: &[u8] = &[137,80,78,71,13,10,26,10,0,0,0,13,73,72,68,82,0,0,0,16,0,0,0,8,8,6,0,0,0,240,118,127,151,0,0,0,26,73,68,65,84,120,156,99,248,143,0,12,56,48,46,0,150,103,64,23,24,53,96,40,26,0,0,53,179,190,80,141,252,69,136,0,0,0,0,73,69,78,68,174,66,96,130];
fn rendered(region: (u32, u32), atlas: (u32, u32)) -> Fixture {
    let mut f = Fixture::provider(br#"{"providers":[{"type":"bitmap","file":"minecraft:font/original.png","ascent":7,"chars":["ab"]}]}"#);
    let png = f.0.join("assets/minecraft/textures/font/original.png");
    std::fs::create_dir_all(png.parent().unwrap()).unwrap();
    std::fs::write(png, ORIGINAL_BITMAP).unwrap();
    let white = horizon_client_render::hud::Rect { u0: 0.0, v0: 0.0, u1: 1.0,
        v1: 1.0, w: 1, h: 1 };
    f.1 = Font::load(&f.0, &std::collections::HashMap::new(), &[], atlas, region, white);
    f
}
fn patches(font: &Font) -> Vec<(u32, u32, u32, u32, Vec<u8>)> {
    font.take_patches().into_iter().map(|p| (p.x, p.y, p.width, p.height, p.rgba)).collect()
}
fn compare_cache(region: (u32, u32), atlas: (u32, u32), full: bool) {
    let old = rendered(region, atlas);
    let new = rendered(region, atlas);
    let pages = [text("ab\n§la", 1)];
    let old_lines = old.1.split(&pages[0], 182.0);
    let new_lines = Book::new(&pages).page(&new.1, 0, 182.0).unwrap();
    assert!(matches!(new_lines, Lines::Shared(_)));
    assert_eq!(old_lines, new_lines.as_slice());
    let before = patches(&old.1);
    assert_eq!(before.len(), if full { 1 } else { 2 }); // Prove the intended allocator branch ran.
    assert_eq!((before[0].0, before[0].1, before[0].2, before[0].3), (0, 0, 8, 8));
    assert_eq!(before, patches(&new.1));
    // Compare a selected row's exact old traversal without rerunning split.
    let line = old_lines[1].resolve_legacy();
    let mut remaining = 0.0;
    let mut link = None;
    for (character, style) in line.chars() {
        remaining -= old.1.advance(character, style);
        if remaining < 0.0 { link = style.click.clone(); break; }
    }
    assert_eq!(link, new_lines.click(&new.1, 0.0, 1));
    assert_eq!(patches(&old.1), patches(&new.1));
    // Observe subsequent cache reads, including native missing-glyph fallback
    // when the deliberately tiny region has no room. No GPU upload is run.
    assert!(old.1.glyph('a').rect.is_some());
    assert_eq!(old.1.glyph('b').rect.is_none(), full);
    assert_eq!(old.1.glyph('b').advance, if full { 6.0 } else { 7.0 });
    assert!(new.1.glyph('a').rect.is_some());
    assert_eq!(new.1.glyph('b').rect.is_none(), full);
    assert_eq!(new.1.glyph('b').advance, if full { 6.0 } else { 7.0 });
    for c in ['a', 'b', 'a'] {
        let a = old.1.glyph(c);
        let b = new.1.glyph(c);
        assert_eq!((a.advance, a.rect.is_some()), (b.advance, b.rect.is_some()));
    }
    assert_eq!(patches(&old.1), patches(&new.1));
}
#[test]
fn actual_cpu_font_patch_order_matches_split_then_selected_hit() {
    compare_cache((0, 32), (64, 32), false);
}
#[test]
fn actual_full_cpu_font_region_preserves_native_missing_glyph_fallback() {
    compare_cache((0, 8), (9, 8), true);
}
