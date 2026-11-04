// The wire format intentionally supports only literal text, book page links,
// and this guide's own drawing metadata; server commands and URLs are excluded.
use horizon_client::text::{ClickEvent, Style, Text};
const ROOT: &str = "tconstruct:materials-book-root";
const LIMIT: usize = 262_144;

fn root(pages: &[Text]) -> bool {
    pages.first().and_then(|p| p.spans.first()).and_then(|s| s.style.insertion.as_deref()) == Some(ROOT)
}

fn book_id(pages: &[Text]) -> Option<&'static str> {
    if !root(pages) { return None; }
    let title = pages.first()?.spans.first()?.text.as_str();
    if title.starts_with("Materials & You\n") { Some("materials") }
    else if title.starts_with("Puny Smelting\n") { Some("puny") }
    else if title.starts_with("Mighty Smelting\n") { Some("mighty") }
    else if title.starts_with("Encyclopedia\n") { Some("encyclopedia") }
    else { None }
}

pub(crate) fn encode_reply(sequence: u64, pages: &[Text]) -> Option<Vec<u8>> {
    book_id(pages)?;
    let body = encode(pages)?;
    let mut result = Vec::with_capacity(8 + body.len());
    result.extend_from_slice(&sequence.to_be_bytes());
    result.extend_from_slice(&body);
    Some(result)
}

pub(crate) fn decode_reply(sequence: u64, current: &[Text], bytes: &[u8]) -> Option<Vec<Text>> {
    let id = book_id(current)?;
    let (header, body) = bytes.split_at_checked(8)?;
    if u64::from_be_bytes(header.try_into().ok()?) != sequence { return None; }
    let pages = decode(body)?;
    (book_id(&pages) == Some(id)).then_some(pages)
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
    fn replies_match_both_open_request_and_book() {
        let root = |title: &str| Text::styled(format!("{title}\n\n"), Style { insertion: Some(ROOT.into()), ..Default::default() });
        let puny = vec![root("Puny Smelting")];
        let mighty = vec![root("Mighty Smelting")];
        let reply = encode_reply(42, &puny).unwrap();
        assert!(decode_reply(42, &puny, &reply).is_some());
        assert!(decode_reply(43, &puny, &reply).is_none());
        assert!(decode_reply(42, &mighty, &reply).is_none());
        let encyclopedia = vec![Text::styled("Encyclopedia\n", Style { insertion: Some(ROOT.into()), ..Default::default() })];
        let encyclopedia_reply = encode_reply(44, &encyclopedia).unwrap();
        assert!(decode_reply(44, &encyclopedia, &encyclopedia_reply).is_some());
        assert!(decode_reply(44, &puny, &encyclopedia_reply).is_none());
        assert!(decode_reply(45, &encyclopedia, &encyclopedia_reply).is_none());
        assert!(decode_reply(42, &puny, &[0; 7]).is_none());
    }
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
    fn page_and_byte_limits_are_enforced_at_the_boundary() {
        let root = || Text::styled("contents", Style { insertion: Some(ROOT.into()), ..Default::default() });
        let mut page_limited = vec![root()];
        page_limited.extend((1..512).map(|_| Text::literal("page")));
        let wire = encode(&page_limited).unwrap();
        assert_eq!(decode(&wire).unwrap().len(), 512);
        page_limited.push(Text::literal("one too many"));
        assert!(encode(&page_limited).is_none());

        // Find the last valid byte using ordinary page text, each span below the
        // decoder's per-span limit. This checks the actual serializer overhead.
        let mut byte_limited = vec![root()];
        loop {
            byte_limited.push(Text::literal("x".repeat(4000)));
            if encode(&byte_limited).is_none() { byte_limited.pop(); break; }
        }
        byte_limited.push(Text::literal(""));
        let mut low = 0;
        let mut high = 4000;
        while low < high {
            let middle = (low + high + 1) / 2;
            *byte_limited.last_mut().unwrap() = Text::literal("x".repeat(middle));
            if encode(&byte_limited).is_some() { low = middle; } else { high = middle - 1; }
        }
        *byte_limited.last_mut().unwrap() = Text::literal("x".repeat(low));
        let wire = encode(&byte_limited).unwrap();
        assert_eq!(wire.len(), LIMIT);
        assert!(decode(&wire).is_some());
        *byte_limited.last_mut().unwrap() = Text::literal("x".repeat(low + 1));
        assert!(encode(&byte_limited).is_none());
    }
    #[test]
    fn rejects_oversize_and_unmarked_pages() {
        assert!(decode(&vec![0; LIMIT + 1]).is_none());
        assert!(decode(br#"[[{"text":"not the book"}]]"#).is_none());
    }
}
