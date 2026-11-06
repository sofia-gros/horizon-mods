use crate::{font::Font, guide_layout as query, pluto_guide_text as codec};
use pluto_inventory_api::guide::document::{Document, Span, Style};
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "guide_text_baseline.rs"]
mod baseline;

// Original test data, not a copied/prepared Minecraft asset tree. The real
// official Font parser and metric methods execute; no window or GPU is created.
struct Fixture(std::path::PathBuf, Font);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "pluto-font-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let path = root.join("assets/minecraft/font/default.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, include_bytes!("guide_font_provider.json")).unwrap();
        let font = Font::metrics(&root);
        Self(root, font)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn doc(text: &str) -> Document {
    Document::new(
        vec![vec![Span {
            text: text.into(),
            style: Style::default(),
        }]],
        |_| true,
    )
    .unwrap()
}
fn raw(lines: &[crate::text::Text]) -> Vec<String> {
    lines
        .iter()
        .map(|t| t.spans.iter().map(|s| s.text.as_str()).collect())
        .collect()
}

#[test]
fn actual_official_metrics_parse_original_provider_without_renderer_assets() {
    let fixture = Fixture::new();
    let font = &fixture.1;
    assert_eq!(font.advance('a', &Default::default()), 3.0);
    assert_eq!(font.advance('😀', &Default::default()), 7.0);
    assert_eq!(
        font.advance(
            'a',
            &crate::text::Style {
                bold: Some(true),
                ..Default::default()
            }
        ),
        4.0
    );
    assert_eq!(font.width("ab"), 8.0);
    assert!(font.take_patches().is_empty());
}

#[test]
fn exact_raw_native_wrapping_preserves_words_empty_and_trailing_lines() {
    let fixture = Fixture::new();
    for (text, width, expected) in [
        ("ab ab\n", 8.0, vec!["ab", "ab", ""]),
        ("", 8.0, vec![""]),
        ("\n", 8.0, vec!["", ""]),
        ("ab", 1.0, vec!["a", "b"]),
    ] {
        let document = doc(text);
        let native = codec::to_native(&document);
        let wrapped = query::wrap(&fixture.1, &document, 0, width).unwrap();
        assert_eq!(raw(wrapped.native_lines()), expected);
        assert_eq!(wrapped.native_lines(), fixture.1.split(&native[0], width));
        for (i, line) in wrapped.native_lines().iter().enumerate() {
            assert_eq!(wrapped.width(i), Some(fixture.1.width_text(line)));
        }
    }
}

#[test]
fn logical_scalar_hit_uses_half_open_edges_and_preserves_admitted_style() {
    let fixture = Fixture::new();
    let document = Document::new(
        vec![
            vec![Span {
                text: "a😀\nb".into(),
                style: Style {
                    page: Some(2),
                    metadata: Some("other:opaque".into()),
                    color: Some(0x123456),
                    ..Default::default()
                },
            }],
            vec![],
        ],
        |_| true,
    )
    .unwrap();
    let wrapped = query::wrap(&fixture.1, &document, 0, 100.0).unwrap();
    let a = wrapped.hit(0.0, 0.0, 9.0, 0, 2).unwrap().unwrap();
    assert_eq!((a.scalar, a.character), (0, 'a'));
    let emoji = wrapped.hit(3.0, 8.99, 9.0, 0, 2).unwrap().unwrap();
    assert_eq!((emoji.scalar, emoji.character), (1, '😀'));
    assert_eq!(emoji.style, document.pages()[0][0].style);
    assert!(wrapped.hit(10.0, 0.0, 9.0, 0, 2).unwrap().is_none());
    assert_eq!(wrapped.hit(0.0, 9.0, 9.0, 0, 2).unwrap().unwrap().line, 1);
    assert_eq!(
        wrapped.hit(0.0, 0.0, 9.0, 1, 1).unwrap().unwrap().character,
        'b'
    );
    assert!(wrapped.hit(0.0, 9.0, 9.0, 0, 1).unwrap().is_none());
    assert!(wrapped.hit(-0.01, 0.0, 9.0, 0, 2).unwrap().is_none());
    assert!(wrapped.hit(0.0, -0.01, 9.0, 0, 2).unwrap().is_none());
}

#[test]
fn legacy_codes_are_not_resolved_before_the_native_raw_split() {
    let fixture = Fixture::new();
    let wrapped = query::wrap(&fixture.1, &doc("§la"), 0, 6.0).unwrap();
    assert_eq!(raw(wrapped.native_lines()), vec!["§l", "a"]);
    assert!(wrapped.hit(0.0, 0.0, 9.0, 0, 2).unwrap().is_none());
    assert_eq!(
        wrapped
            .hit(0.0, 9.0, 9.0, 0, 2)
            .unwrap()
            .unwrap()
            .style
            .bold,
        None
    );
    let wrapped = query::wrap(&fixture.1, &doc("§la§rb"), 0, 100.0).unwrap();
    assert_eq!(wrapped.width(0), Some(9.0));
    assert_eq!(
        wrapped
            .hit(3.9, 0.0, 9.0, 0, 1)
            .unwrap()
            .unwrap()
            .style
            .bold,
        Some(true)
    );
    let b = wrapped.hit(4.0, 0.0, 9.0, 0, 1).unwrap().unwrap();
    assert_eq!(b.character, 'b');
    assert_ne!(b.style.bold, Some(true));
}

#[test]
fn bidi_measurement_does_not_change_logical_hit_order_or_skip_zero_advance() {
    let fixture = Fixture::new();
    let document = doc("zאב");
    let wrapped = query::wrap(&fixture.1, &document, 0, 100.0).unwrap();
    let hit = wrapped.hit(0.0, 0.0, 9.0, 0, 1).unwrap().unwrap();
    assert_eq!((hit.scalar, hit.character), (1, 'א'));
    assert_eq!(
        wrapped.width(0),
        Some(fixture.1.width_text(&codec::to_native(&document)[0]))
    );
}

#[test]
fn finite_geometry_and_window_overflow_are_explicitly_rejected() {
    let fixture = Fixture::new();
    for width in [0.0, -1.0, f32::INFINITY, f32::NAN] {
        assert!(matches!(
            query::wrap(&fixture.1, &doc("a"), 0, width),
            Err(query::Error::Width)
        ));
    }
    assert!(matches!(
        query::wrap(&fixture.1, &doc("a"), 1, 1.0),
        Err(query::Error::Page)
    ));
    let wrapped = query::wrap(&fixture.1, &doc("a\nb"), 0, 10.0).unwrap();
    for height in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            wrapped.hit(0.0, 0.0, height, 0, 1),
            Err(query::Error::LineHeight)
        );
    }
    assert_eq!(
        wrapped.hit(f32::NAN, 0.0, 1.0, 0, 1),
        Err(query::Error::Point)
    );
    assert_eq!(
        wrapped.hit(0.0, f32::INFINITY, 1.0, 0, 1),
        Err(query::Error::Point)
    );
    assert_eq!(
        wrapped.hit(0.0, 1.0, 1.0, usize::MAX, 2),
        Err(query::Error::Window)
    );
    assert_eq!(
        wrapped.hit(0.0, 0.0, 1.0, 0, query::MAX_LINES + 1),
        Err(query::Error::Window)
    );
    assert!(wrapped.hit(0.0, 0.0, 1.0, 0, 0).unwrap().is_none());
}

#[test]
fn style_amplification_and_scalar_budgets_apply_before_native_split() {
    let fixture = Fixture::new();
    let text = "a".repeat(4096);
    let expensive = Document::new(
        vec![vec![Span {
            text: text.clone(),
            style: Style {
                metadata: Some("m".repeat(8192)),
                ..Default::default()
            },
        }]],
        |_| true,
    )
    .unwrap();
    assert!(matches!(
        query::wrap(&fixture.1, &expensive, 0, 1.0),
        Err(query::Error::Budget)
    ));
    let excess = Document::new(
        vec![vec![
            Span {
                text,
                style: Style::default(),
            },
            Span {
                text: "a".into(),
                style: Style::default(),
            },
        ]],
        |_| true,
    )
    .unwrap();
    assert!(matches!(
        query::wrap(&fixture.1, &excess, 0, 1.0),
        Err(query::Error::Budget)
    ));
}

#[test]
fn converter_refactor_matches_frozen_production_for_values_and_rejection_order() {
    use crate::text::{ClickEvent, Style as NativeStyle, Text};
    let mut cases = vec![Text::empty(), Text::literal("§l日本語😀")];
    for style in [
        NativeStyle {
            insertion: Some("example:meta".into()),
            bold: Some(false),
            click: Some(ClickEvent::ChangePage(2)),
            ..Default::default()
        },
        NativeStyle {
            click: Some(ClickEvent::OpenUrl("https://example.test".into())),
            ..Default::default()
        },
        NativeStyle {
            click: Some(ClickEvent::ChangePage(-1)),
            ..Default::default()
        },
        NativeStyle {
            font: Some("example:font".into()),
            ..Default::default()
        },
        NativeStyle {
            insertion: Some("m".repeat(8193)),
            ..Default::default()
        },
    ] {
        cases.push(Text::styled("value", style.clone()));
        cases.push(Text::styled("x".repeat(4097), style));
    }
    for page in cases {
        let pages = [page, Text::empty()];
        let old = baseline::from_native(&pages, |_| true);
        let new = codec::from_native(&pages, |_| true);
        assert_eq!(new, old);
        if let Ok(document) = new {
            assert_eq!(codec::to_native(&document), baseline::to_native(&document));
        }
        assert_eq!(
            codec::from_native(&pages, |_| false),
            baseline::from_native(&pages, |_| false)
        );
    }
}
