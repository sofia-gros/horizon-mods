fn tconstruct_materials_book_request(game: &mut Game<'_>, host: &mut LanHost, conn: u32, packet: &S) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let S::CustomPayload(message) = packet else { return Continue(()); };
    if message.channel != "tconstruct:materials_book" { return Continue(()); }
    let Some(guest) = host.hub.guests.iter().find(|guest| guest.conn == conn) else { return Break(()); };
    let held = match message.data.as_slice() {
        [0] => guest.inventory.get(guest.selected).and_then(Option::as_ref),
        [1] => guest.offhand.as_ref(),
        _ => None,
    };
    if !held.is_some_and(|item| &*item.item == "tconstruct:materials_and_you") { return Break(()); }
    let path = game.options.resources.join("assets/tconstruct/materials-and-you.zip");
    let Some(pages) = super::books::tconstruct_materials_book::source_pages(game, &path) else { return Break(()); };
    if pages.len() > 512 { return Break(()); }
    let pages: Vec<Vec<serde_json::Value>> = pages.iter().map(|page| page.spans.iter().map(|span| {
        let click = match &span.style.click { Some(horizon_client::text::ClickEvent::ChangePage(page)) => Some(*page), _ => None };
        serde_json::json!({"text":span.text, "page":click, "insertion":span.style.insertion})
    }).collect()).collect();
    let Ok(data) = serde_json::to_vec(&pages) else { return Break(()); };
    if data.len() > 262_144 { return Break(()); }
    guest.send(&C::CustomPayload(horizon_protocol::packets::configuration::CustomPayload {
        channel: "tconstruct:materials_book".into(), data,
    }));
    Break(())
}
