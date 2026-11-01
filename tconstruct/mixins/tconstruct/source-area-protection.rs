// Ask the existing server permission check about each potential extra block.
pub(super) fn protected(game: &super::Game<'_>, host: &super::LanHost, conn: u32, pos: [i32;3], face: [i32;3]) -> Option<std::collections::HashSet<[i32;3]>> {
    let gi = host.hub.guests.iter().position(|guest| guest.conn == conn)?;
    let yaw = host.hub.guests[gi].yaw;
    let mut protected = std::collections::HashSet::new();
    for extra in crate::game::tconstruct_source_area_shape::plane(pos, face, yaw)? {
        if game.under_spawn_protection(host, gi, extra) { protected.insert(extra); }
    }
    Some(protected)
}
