// Recheck at the authoritative break, including delayed server destruction.
fn tconstruct_area_guest_destroy(game: &mut crate::game::Game<'_>, host: &crate::game::lan::LanHost, conn: u32, pos: [i32;3]) {
    let face = game.world.chunks.__horizon_mixins_tconstruct_area_guest_hits().get(&conn)
        .filter(|(target, _, _)| *target == pos).map(|(_, face, _)| *face);
    if let Some((face, protected)) = face.and_then(|face|
        self::tconstruct_source_area_protection::protected(game, host, conn, pos, face).map(|blocked| (face, blocked))) {
        game.world.chunks.__horizon_mixins_tconstruct_area_guest_hits_mut().insert(conn, (pos, face, protected));
    }
}
