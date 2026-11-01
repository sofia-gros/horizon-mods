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
        serde_json::json!({"text":span.text, "page":click, "insertion":span.style.insertion})
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
            let insertion = span.get("insertion").and_then(|v| v.as_str()).filter(|v| v.len() <= 4096 && (v.starts_with("tconstruct:book:") || *v == ROOT)).map(str::to_owned);
            text.push(Text::styled(value, Style { underlined: click.as_ref().map(|_| true), click, insertion, ..Default::default() }));
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
        let second = Text::styled("image", Style { insertion: Some("tconstruct:book:{\"image\":\"x\"}".into()), click: Some(ClickEvent::RunCommand("/test".into())), ..Default::default() });
        let result = decode(&encode(&[first, second]).unwrap()).unwrap();
        assert_eq!(result[0].spans[1].style.click, Some(ClickEvent::ChangePage(2)));
        assert_eq!(result[1].spans[0].style.click, None);
        assert_eq!(result[1].spans[0].style.insertion.as_deref(), Some("tconstruct:book:{\"image\":\"x\"}"));
    }
    #[test]
    fn rejects_oversize_and_unmarked_pages() {
        assert!(decode(&vec![0; LIMIT + 1]).is_none());
        assert!(decode(br#"[[{"text":"not the book"}]]"#).is_none());
    }
}
