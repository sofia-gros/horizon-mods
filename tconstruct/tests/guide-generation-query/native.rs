// Includes the unchanged actual Font/Text fixture helpers and its original 12
// tests. A future runner must seal both inputs and select this new six-test
// module separately. No mock Text, Font, Game or renderer is introduced.
include!("../guide-text-migration/native.rs");

mod generation_tests {
    use super::*;

    // Frozen native source-generation oracle, with only Game's font receiver
    // replaced by the same borrowed Font. No Pluto or new consumer call here.
    fn old_paginate(font: &Font, text: &Text, width: f32, rows: usize) -> Vec<Text> {
        let resolved = text.resolve_legacy();
        let lines = font.split(&resolved, width);
        lines.chunks(rows.max(1)).map(|chunk| {
            let mut page = Text::empty();
            for (i, line) in chunk.iter().enumerate() {
                if i > 0 { page.push("\n"); }
                page.push(line.clone());
            }
            page
        }).collect()
    }
    fn compare_pages(font: &Font, text: &Text, width: f32, rows: usize) {
        assert_eq!(crate::consumer::paginate(font, text, width, rows),
                   old_paginate(font, text, width, rows));
    }
    #[test]
    fn legacy_resolution_empty_spans_and_row_boundaries_match_native() {
        let f = Fixture::new();
        let mut styled = Text::literal("§la§rb zאב\n😀\n\nab ab");
        styled.spans.insert(0, Span { text: String::new(), style: Style {
            italic: Some(true), insertion: Some("opaque:empty".into()), ..Default::default()
        }});
        for value in [styled, Text::literal(""), Text::literal("\n\n"), text("ab\na", 1)] {
            for width in [1.0, 8.0, 182.0] {
                for rows in [0, 1, 2, 17, usize::MAX] { compare_pages(&f.1, &value, width, rows); }
            }
        }
    }
    #[test]
    fn unsupported_styles_and_cross_page_actions_keep_native_fallback() {
        let f = Fixture::new();
        for style in [
            Style { click: Some(ClickEvent::RunCommand("/unchanged".into())), ..Default::default() },
            Style { click: Some(ClickEvent::ChangePage(99)), ..Default::default() },
            Style { click: Some(ClickEvent::ChangePage(0)), ..Default::default() },
            Style { font: Some("minecraft:default".into()), ..Default::default() },
            Style { hover: Some(Box::new(Text::literal("hover"))), ..Default::default() },
            Style { shadow_color: Some(0xff00ff00), ..Default::default() },
        ] { compare_pages(&f.1, &Text::styled("ab\na", style), 8.0, 1); }
    }
    #[test]
    fn rejected_document_and_work_budgets_do_not_erase_content() {
        let f = Fixture::new();
        let mut many = Text::empty();
        many.spans = (0..33).map(|i| Span { text: "a".into(), style: Style {
            color: Some(i), ..Default::default()
        }}).collect();
        let huge = Text::styled("a", Style { insertion: Some("x".repeat(8193)), ..Default::default() });
        let amplified = Text::styled("a".repeat(130), Style {
            insertion: Some("x".repeat(8192)), ..Default::default()
        });
        for value in [many, huge, amplified, Text::literal("a".repeat(4097))] {
            compare_pages(&f.1, &value, 182.0, 17);
        }
    }
    #[test]
    fn nonfinite_widths_and_native_metrics_keep_original_arithmetic() {
        let f = Fixture::provider(br#"{"providers":[{"type":"space","advances":{"a":-3,"b":1e100}}]}"#);
        assert!(f.1.advance('a', &Style::default()) < 0.0);
        assert!(!f.1.advance('b', &Style::default()).is_finite());
        for width in [0.0, -1.0, f32::NAN, f32::INFINITY, 182.0] {
            compare_pages(&f.1, &Text::literal("ab\nab"), width, 1);
        }
    }
    fn compare_generation_cache(region: (u32, u32), atlas: (u32, u32), full: bool) {
        let old = rendered(region, atlas);
        let new = rendered(region, atlas);
        let input = Text::literal("ab\n§la\na");
        let expected = old_paginate(&old.1, &input, 182.0, 1);
        let actual = crate::consumer::paginate(&new.1, &input, 182.0, 1);
        assert_eq!(actual, expected);
        let emitted = patches(&old.1);
        assert_eq!(emitted.len(), if full { 1 } else { 2 });
        assert_eq!((emitted[0].0, emitted[0].1, emitted[0].2, emitted[0].3), (0, 0, 8, 8));
        assert_eq!(emitted, patches(&new.1));
        for font in [&old.1, &new.1] {
            assert!(font.glyph('a').rect.is_some());
            assert_eq!(font.glyph('b').rect.is_none(), full);
            assert_eq!(font.glyph('b').advance, if full { 6.0 } else { 7.0 });
        }
        assert_eq!(patches(&old.1), patches(&new.1));
    }
    #[test]
    fn actual_cpu_generation_patch_order_matches_native() {
        compare_generation_cache((0, 32), (64, 32), false);
    }
    #[test]
    fn actual_full_cpu_region_keeps_native_missing_glyph_fallback() {
        compare_generation_cache((0, 8), (9, 8), true);
    }
}
