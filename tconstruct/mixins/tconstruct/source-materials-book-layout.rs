pub(super) fn append_parts(out: &mut String, parts: &[serde_json::Value], separate: bool) {
    for part in parts {
        let Some(value) = part.as_str().or_else(|| part.get("text").and_then(|x| x.as_str())) else { continue; };
        if !out.is_empty() {
            if part.get("paragraph").and_then(|v| v.as_bool()) == Some(true) { out.push_str("\n\n"); }
            else if separate { out.push('\n'); }
        }
        out.push_str(value);
        if part.get("linebreak").and_then(|v| v.as_bool()) == Some(true) { out.push('\n'); }
    }
}

// A source material has two paired rows. Split tall stat lists without
// dropping values, then leave only the space that is actually free for flavor.
pub(super) fn material_panel_count(lengths: &[Option<usize>]) -> usize {
    lengths.iter().flatten().map(|len| (len + 4) / 5).max().unwrap_or(1).max(1)
}

pub(super) fn material_line_range(length: usize, page: usize) -> std::ops::Range<usize> {
    let start = page.saturating_mul(5).min(length);
    start..start.saturating_add(5).min(length)
}

// Keep wrapped lines with their label whenever they fit in one panel.
pub(super) fn material_line_ranges(continuation: &[bool]) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < continuation.len() {
        let mut end = (start + 5).min(continuation.len());
        if end < continuation.len() && continuation[end] {
            while end > start && continuation[end] { end -= 1; }
            if end == start { end = (start + 5).min(continuation.len()); }
        }
        ranges.push(start..end);
        start = end;
    }
    ranges
}

pub(super) fn material_flavor_rows(lengths: &[Option<usize>]) -> usize {
    let row = |start: usize| lengths.iter().skip(start).take(2).flatten().copied().max().map_or(0, |count| 15 + count * 10);
    178usize.saturating_sub(28 + row(0) + row(2) + 5) / 9
}

// IToolStat.formatColored uses the source positive-modulo HSV mapping.
pub(super) fn bonus_color(value: f64) -> u32 {
    let hue = (0.5 + value).rem_euclid(2.0) / 1.5;
    let h = (hue * 6.0).rem_euclid(6.0);
    let i = h.floor() as u8;
    let f = h - f64::from(i);
    let v = 0.75;
    let q = v * (1.0 - f);
    let t = v * f;
    let (r, g, b) = match i {
        0 => (v, t, 0.0), 1 => (q, v, 0.0), 2 => (0.0, v, t),
        3 => (0.0, q, v), 4 => (t, 0.0, v), _ => (v, 0.0, q),
    };
    ((r * 255.0) as u32) << 16 | ((g * 255.0) as u32) << 8 | (b * 255.0) as u32
}

// The graphic metadata is sent in the bounded book payload. Only text and
// visual styles are accepted here; actions remain in the main wire format.
pub(super) fn encode_runs(text: &horizon_client::text::Text) -> Vec<serde_json::Value> {
    text.spans.iter().map(|span| {
        let mut value = serde_json::Map::new();
        value.insert("text".into(), serde_json::json!(span.text));
        if let Some(color) = span.style.color { value.insert("color".into(), serde_json::json!(color)); }
        for (key, flag) in [("bold", span.style.bold), ("italic", span.style.italic), ("underlined", span.style.underlined), ("strikethrough", span.style.strikethrough), ("obfuscated", span.style.obfuscated)] {
            if let Some(flag) = flag { value.insert(key.into(), serde_json::json!(flag)); }
        }
        serde_json::Value::Object(value)
    }).collect()
}

pub(super) fn decode_runs(value: &serde_json::Value) -> Option<horizon_client::text::Text> {
    use horizon_client::text::{Style, Text};
    let spans = value.as_array().filter(|v| v.len() <= 64)?;
    let mut out = Text::empty();
    for span in spans {
        let text = span.get("text")?.as_str().filter(|v| v.len() <= 4096)?;
        let style = Style {
            color: span.get("color").and_then(|v| v.as_u64()).and_then(|v| u32::try_from(v).ok()),
            bold: span.get("bold").and_then(|v| v.as_bool()),
            italic: span.get("italic").and_then(|v| v.as_bool()),
            underlined: span.get("underlined").and_then(|v| v.as_bool()),
            strikethrough: span.get("strikethrough").and_then(|v| v.as_bool()),
            obfuscated: span.get("obfuscated").and_then(|v| v.as_bool()),
            ..Default::default()
        };
        out.push(Text::styled(text, style));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{append_parts, bonus_color, decode_runs, encode_runs, material_flavor_rows, material_line_range, material_line_ranges, material_panel_count};
    #[test]
    fn material_wrapped_values_stay_with_their_label() {
        assert_eq!(material_line_ranges(&[false, false, false, false, false, true, false]), vec![0..4, 4..7]);
        assert_eq!(material_line_ranges(&[false; 6]), vec![0..5, 5..6]);
        assert_eq!(material_line_ranges(&[false, true, true, true, true, true]), vec![0..5, 5..6]);
    }
    #[test]
    fn material_panels_preserve_overflow_and_reserve_flavor_space() {
        assert_eq!(material_panel_count(&[Some(7), Some(2), Some(1)]), 2);
        assert_eq!(material_panel_count(&[Some(0), None, None]), 1);
        assert_eq!(material_flavor_rows(&[Some(5), Some(2), Some(5)]), 1);
        assert_eq!(material_flavor_rows(&[None, None, None]), 16);
        let original: Vec<_> = (0..12).collect();
        let reconstructed: Vec<_> = (0..material_panel_count(&[Some(original.len())])).flat_map(|page| original[material_line_range(original.len(), page)].iter().copied()).collect();
        assert_eq!(reconstructed, original);
    }
    #[test]
    fn source_bonus_hue_maps_negative_neutral_and_positive() {
        assert_eq!(bonus_color(-0.5), 0xbf0000);
        assert_eq!(bonus_color(0.0), 0x00bf00);
        assert_eq!(bonus_color(0.5), 0x0000bf);
    }
    #[test]
    fn entity_reference_bullets_keep_line_breaks() {
        let mut out = String::new();
        append_parts(&mut out, &[
            serde_json::json!({"text":"Silverfish - Seared Stone", "linebreak":true}),
            serde_json::json!({"text":"Villagers - Emerald", "linebreak":true}),
            serde_json::json!({"text":"Skeletons - Milk"}),
        ], false);
        assert_eq!(out, "Silverfish - Seared Stone\nVillagers - Emerald\nSkeletons - Milk");
    }
    #[test]
    fn inline_runs_and_paragraphs_keep_source_boundaries() {
        let mut out = String::new();
        append_parts(&mut out, &[serde_json::json!({"text":"one "}), serde_json::json!({"text":"two"}), serde_json::json!({"text":"three", "paragraph":true})], false);
        assert_eq!(out, "one two\n\nthree");
    }
    #[test]
    fn styled_graphic_text_survives_serialization() {
        use horizon_client::text::{Style, Text};
        let text = Text::styled("emphasis", Style { bold: Some(true), color: Some(0x123456), ..Default::default() });
        let decoded = decode_runs(&serde_json::json!(encode_runs(&text))).unwrap();
        assert_eq!(decoded.spans[0].style.bold, Some(true));
        assert_eq!(decoded.spans[0].style.color, Some(0x123456));
    }
    #[test]
    fn source_legacy_emphasis_survives_graphic_payload() {
        let text = horizon_client::text::Text::literal("§lTHRUUL§r and §oPuny§r").resolve_legacy();
        let decoded = decode_runs(&serde_json::json!(encode_runs(&text))).unwrap();
        assert!(decoded.spans.iter().any(|span| span.text.contains("THRUUL") && span.style.bold == Some(true)));
        assert!(decoded.spans.iter().any(|span| span.text.contains("Puny") && span.style.italic == Some(true)));
    }
    #[test]
    fn properties_are_separate_lines() {
        let mut out = String::new();
        append_parts(&mut out, &[serde_json::json!("first"), serde_json::json!("second")], true);
        assert_eq!(out, "first\nsecond");
    }
}
