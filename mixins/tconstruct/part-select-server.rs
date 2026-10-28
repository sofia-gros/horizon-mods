fn select_part_recipe(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, window: i32, id: i32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(g) = host.hub.guests.iter().find(|g| g.conn == conn) else { return Continue(()); };
    let Some(menu) = g.menu.as_ref().filter(|m| m.window == window) else { return Continue(()); };
    let Some(key) = menu.pos else { return Continue(()); };
    if &*game.res.facts().get(game.world.block(key.0, key.1, key.2)).name != "tconstruct:part_builder" { return Continue(()); }
    if !g.spectator() && menu.slots.still_valid(&game.world, g, menu.pos) && (0..3).contains(&id)
        && game.world.block_entities.get(&key).and_then(|e| e.slots().first()).and_then(Option::as_ref).is_some_and(|s| &*s.item == "tconstruct:pattern") {
        if let Some(pattern) = crate::game::tconstruct_tx::preview_pattern(id as i16) {
            game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().insert(key, pattern.to_owned());
        }
    }
    Break(())
}
