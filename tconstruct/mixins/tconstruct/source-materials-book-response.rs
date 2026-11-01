fn tconstruct_materials_book_response(game: &mut Game<'_>, packet: &C) -> std::ops::ControlFlow<Result<()>> {
    use std::ops::ControlFlow::{Break, Continue};
    let C::CustomPayload(message) = packet else { return Continue(()); };
    if message.channel != "tconstruct:materials_book" { return Continue(()); }
    let Some(super::books::BookScreen::View { pages, page, lectern: None }) = game.book.as_mut() else { return Break(Ok(())); };
    if pages.first().and_then(|p| p.spans.first()).and_then(|s| s.style.insertion.as_deref()) != Some("tconstruct:materials-book-root") || message.data.len() > 262_144 { return Break(Ok(())); }
    let Ok(wire) = serde_json::from_slice::<Vec<Vec<serde_json::Value>>>(&message.data) else { return Break(Ok(())); };
    if wire.is_empty() || wire.len() > 512 || wire.iter().any(|p| p.len() > 32) { return Break(Ok(())); }
    let count = wire.len();
    let mut decoded = Vec::with_capacity(count);
    for part in wire {
        let mut text = horizon_client::text::Text::empty();
        for span in part {
            let Some(value) = span.get("text").and_then(|v| v.as_str()).filter(|v| v.len() <= 4096) else { return Break(Ok(())); };
            let click = span.get("page").and_then(|v| v.as_i64()).filter(|v| *v > 0 && *v <= count as i64).map(|v| horizon_client::text::ClickEvent::ChangePage(v as i32));
            let insertion = span.get("insertion").and_then(|v| v.as_str()).filter(|v| v.len() <= 4096 && (v.starts_with("tconstruct:book:") || *v == "tconstruct:materials-book-root")).map(str::to_owned);
            let style = horizon_client::text::Style { underlined: click.as_ref().map(|_| true), click, insertion, ..Default::default() };
            text.push(horizon_client::text::Text::styled(value, style));
        }
        decoded.push(text);
    }
    if decoded.first().and_then(|p| p.spans.first()).and_then(|s| s.style.insertion.as_deref()) != Some("tconstruct:materials-book-root") { return Break(Ok(())); }
    *page = (*page).min(decoded.len() - 1);
    *pages = decoded;
    Break(Ok(()))
}
