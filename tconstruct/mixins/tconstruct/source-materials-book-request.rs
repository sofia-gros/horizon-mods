fn tconstruct_materials_book_request(game: &mut Game<'_>, host: &mut LanHost, conn: u32, packet: &S) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let S::CustomPayload(message) = packet else { return Continue(()); };
    if message.channel != "tconstruct:materials_book" { return Continue(()); }
    let Some(guest) = host.hub.guests.iter().find(|guest| guest.conn == conn) else { return Break(()); };
    let (hand, sequence) = match message.data.as_slice() {
        [hand @ (0 | 1)] => (*hand, None),
        [hand @ (0 | 1), bytes @ ..] if bytes.len() == 8 => (*hand, Some(u64::from_be_bytes(bytes.try_into().unwrap()))),
        _ => return Break(()),
    };
    let held = match hand { 0 => guest.inventory.get(guest.selected).and_then(Option::as_ref), 1 => guest.offhand.as_ref(), _ => None };
    let Some(held) = held else { return Break(()); };
    let book = &*held.item;
    if !matches!(book, "tconstruct:puny_smelting" | "tconstruct:mighty_smelting" | "tconstruct:encyclopedia" | "tconstruct:materials_and_you") { return Break(()); }
    let path = game.options.resources.clone();
    let Some(pages) = super::books::tconstruct_materials_book::source_pages(game, &path, book) else { return Break(()); };
    let data = match sequence {
        Some(sequence) => super::books::tconstruct_materials_book_wire::encode_reply(sequence, &pages),
        None => super::books::tconstruct_materials_book_wire::encode(&pages),
    };
    let Some(data) = data else { return Break(()); };
    guest.send(&C::CustomPayload(horizon_protocol::packets::configuration::CustomPayload {
        channel: "tconstruct:materials_book".into(), data,
    }));
    Break(())
}
