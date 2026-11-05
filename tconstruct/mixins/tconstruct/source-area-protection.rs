// Ask the existing server permission check for each potentially affected column.
// Spawn protection is independent of Y; trees can reach above the clicked block.
pub(super) fn protected(game: &super::Game<'_>, host: &super::LanHost, conn: u32, pos: [i32;3], face: [i32;3]) -> Option<std::collections::HashSet<(i32,i32)>> {
    protected_for(game,host,conn,pos,face,false)
}
pub(super) fn protected_for(game: &super::Game<'_>, host: &super::LanHost, conn:u32, pos:[i32;3], face:[i32;3], offhand:bool) -> Option<std::collections::HashSet<(i32,i32)>> {
    let gi = host.hub.guests.iter().position(|guest| guest.conn == conn)?;
    let guest = &host.hub.guests[gi];
    let held=if offhand {guest.offhand.as_ref()} else {guest.inventory.get(guest.selected).and_then(Option::as_ref)};
    let tool=held.map(|stack|stack.item.as_ref()).unwrap_or("");
    let mut protected = std::collections::HashSet::new();
    let radius = match tool { "tconstruct:vein_hammer" => 2, "tconstruct:scythe" => 3, "tconstruct:broad_axe" => 10, _ => 0 };
    if radius > 0 {
        for dx in -radius..=radius { for dz in -radius..=radius {
            if i32::abs(dx) + i32::abs(dz) > radius { continue; }
            let (Some(x), Some(z)) = (pos[0].checked_add(dx), pos[2].checked_add(dz)) else { continue; };
            if game.under_spawn_protection(host, gi, [x,pos[1],z]) { protected.insert((x,z)); }
        }}
    } else {
        let expanded=if tool=="tconstruct:sledge_hammer" {crate::game::tconstruct_expanded::level(&game.world,held?)?} else {0};
        for extra in crate::game::tconstruct_source_area_shape::expanded_plane(pos, face, guest.yaw,expanded)? {
            if game.under_spawn_protection(host, gi, extra) { protected.insert((extra[0],extra[2])); }
        }
    }
    // Glowing can place on the adjacent side of the clicked block, possibly
    // across the spawn-protection boundary even if the click itself is allowed.
    if let (Some(x), Some(y), Some(z)) = (pos[0].checked_add(face[0]), pos[1].checked_add(face[1]), pos[2].checked_add(face[2])) {
        if game.under_spawn_protection(host, gi, [x,y,z]) { protected.insert((x,z)); }
    }
    Some(protected)
}
