// Ask the existing server permission check for each potentially affected column.
// Spawn protection is independent of Y; trees can reach above the clicked block.
pub(super) fn protected(game: &super::Game<'_>, host: &super::LanHost, conn: u32, pos: [i32;3], face: [i32;3]) -> Option<std::collections::HashSet<(i32,i32)>> {
    let gi = host.hub.guests.iter().position(|guest| guest.conn == conn)?;
    let guest = &host.hub.guests[gi];
    let tool = guest.inventory.get(guest.selected).and_then(Option::as_ref).map(|stack| stack.item.as_ref()).unwrap_or("");
    let mut protected = std::collections::HashSet::new();
    let radius = match tool { "tconstruct:vein_hammer" => 2, "tconstruct:broad_axe" => 10, _ => 0 };
    if radius > 0 {
        for dx in -radius..=radius { for dz in -radius..=radius {
            if i32::abs(dx) + i32::abs(dz) > radius { continue; }
            let (Some(x), Some(z)) = (pos[0].checked_add(dx), pos[2].checked_add(dz)) else { continue; };
            if game.under_spawn_protection(host, gi, [x,pos[1],z]) { protected.insert((x,z)); }
        }}
    } else {
        for extra in crate::game::tconstruct_source_area_shape::plane(pos, face, guest.yaw)? {
            if game.under_spawn_protection(host, gi, extra) { protected.insert((extra[0],extra[2])); }
        }
    }
    Some(protected)
}
