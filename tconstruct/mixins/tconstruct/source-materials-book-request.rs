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
    let Some(held) = held else { return Break(()); };
    let puny = &*held.item == "tconstruct:puny_smelting";
    if !puny && &*held.item != "tconstruct:materials_and_you" { return Break(()); }
    let path = game.options.resources.clone();
    let Some(pages) = super::books::tconstruct_materials_book::source_pages(game, &path, puny) else { return Break(()); };
    let Some(data) = super::books::tconstruct_materials_book_wire::encode(&pages) else { return Break(()); };
    guest.send(&C::CustomPayload(horizon_protocol::packets::configuration::CustomPayload {
        channel: "tconstruct:materials_book".into(), data,
    }));
    Break(())
}
