// The wire format intentionally supports only literal text, book page links,
// and this guide's own drawing metadata; server commands and URLs are excluded.
use horizon_client::text::{ClickEvent, Style, Text};
const ROOT: &str = "tconstruct:materials-book-root";
const LIMIT: usize = 262_144;

fn root(pages: &[Text]) -> bool {
    pages.first().and_then(|p| p.spans.first()).and_then(|s| s.style.insertion.as_deref()) == Some(ROOT)
}

pub(crate) fn encode(pages: &[Text]) -> Option<Vec<u8>> {
    if pages.is_empty() || pages.len() > 512 || !root(pages) { return None; }
    let data: Vec<Vec<serde_json::Value>> = pages.iter().map(|page| page.spans.iter().map(|span| {
        let click = match &span.style.click { Some(ClickEvent::ChangePage(page)) => Some(*page), _ => None };
        serde_json::json!({"text":span.text, "page":click, "insertion":span.style.insertion,
            "color":span.style.color, "bold":span.style.bold, "italic":span.style.italic,
            "underlined":span.style.underlined, "strikethrough":span.style.strikethrough,
            "obfuscated":span.style.obfuscated})
    }).collect()).collect();
    let data = serde_json::to_vec(&data).ok()?;
    (data.len() <= LIMIT).then_some(data)
}

pub(crate) fn decode(data: &[u8]) -> Option<Vec<Text>> {
    if data.len() > LIMIT { return None; }
    let wire = serde_json::from_slice::<Vec<Vec<serde_json::Value>>>(data).ok()?;
    if wire.is_empty() || wire.len() > 512 || wire.iter().any(|p| p.len() > 32) { return None; }
    let count = wire.len();
    let mut pages = Vec::with_capacity(count);
    for part in wire {
        let mut text = Text::empty();
        for span in part {
            let value = span.get("text")?.as_str().filter(|v| v.len() <= 4096)?;
            let click = span.get("page").and_then(|v| v.as_i64()).filter(|v| *v > 0 && *v <= count as i64).map(|v| ClickEvent::ChangePage(v as i32));
            let insertion = span.get("insertion").and_then(|v| v.as_str()).filter(|v| v.len() <= 8192 && (v.starts_with("tconstruct:book:") || *v == ROOT || *v == "tconstruct:book-title")).map(str::to_owned);
            let underlined = span.get("underlined").and_then(|v| v.as_bool()).or_else(|| click.as_ref().map(|_| true));
            let color = span.get("color").and_then(|v| v.as_u64()).and_then(|v| u32::try_from(v).ok());
            text.push(Text::styled(value, Style {
                color, bold: span.get("bold").and_then(|v| v.as_bool()),
                italic: span.get("italic").and_then(|v| v.as_bool()), underlined,
                strikethrough: span.get("strikethrough").and_then(|v| v.as_bool()),
                obfuscated: span.get("obfuscated").and_then(|v| v.as_bool()),
                click, insertion, ..Default::default()
            }));
        }
        pages.push(text);
    }
    root(&pages).then_some(pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_links_and_graphics_round_trip_and_other_actions_are_omitted() {
        let mut first = Text::styled("contents", Style { insertion: Some(ROOT.into()), ..Default::default() });
        first.push(Text::styled("chapter", Style { click: Some(ClickEvent::ChangePage(2)), ..Default::default() }));
        let second = Text::styled("image", Style { insertion: Some("tconstruct:book:{\"image\":\"x\"}".into()), color: Some(0x123456), bold: Some(true), underlined: Some(true), click: Some(ClickEvent::RunCommand("/test".into())), ..Default::default() });
        let result = decode(&encode(&[first, second]).unwrap()).unwrap();
        assert_eq!(result[0].spans[1].style.click, Some(ClickEvent::ChangePage(2)));
        assert_eq!(result[1].spans[0].style.click, None);
        assert_eq!(result[1].spans[0].style.color, Some(0x123456));
        assert_eq!(result[1].spans[0].style.bold, Some(true));
        assert_eq!(result[1].spans[0].style.underlined, Some(true));
        assert_eq!(result[1].spans[0].style.insertion.as_deref(), Some("tconstruct:book:{\"image\":\"x\"}"));
    }
    #[test]
    fn bounded_material_panel_metadata_survives_guest_transfer() {
        let root = Text::styled("contents", Style { insertion: Some(ROOT.into()), ..Default::default() });
        let metadata = format!("tconstruct:book:{}", "x".repeat(6000));
        let panel = Text::styled("material", Style { insertion: Some(metadata.clone()), ..Default::default() });
        let pages = decode(&encode(&[root, panel]).unwrap()).unwrap();
        assert_eq!(pages[1].spans[0].style.insertion.as_deref(), Some(metadata.as_str()));
    }
    #[test]
    fn source_title_style_survives_guest_transfer() {
        let root = Text::styled("contents", Style { insertion: Some(ROOT.into()), ..Default::default() });
        let title = Text::styled("Source title", Style { insertion: Some("tconstruct:book-title".into()), underlined: Some(true), color: Some(0x102030), ..Default::default() });
        let pages = decode(&encode(&[root, title]).unwrap()).unwrap();
        assert_eq!(pages[1].spans[0].style.insertion.as_deref(), Some("tconstruct:book-title"));
        assert_eq!(pages[1].spans[0].style.underlined, Some(true));
        assert_eq!(pages[1].spans[0].style.color, Some(0x102030));
    }
    #[test]
    fn rejects_oversize_and_unmarked_pages() {
        assert!(decode(&vec![0; LIMIT + 1]).is_none());
        assert!(decode(br#"[[{"text":"not the book"}]]"#).is_none());
    }
}
