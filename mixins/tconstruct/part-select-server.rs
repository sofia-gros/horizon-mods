fn select_part_recipe(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, window: i32, id: i32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(g) = host.hub.guests.iter().find(|g| g.conn == conn) else { return Continue(()); };
    let Some(menu) = g.menu.as_ref().filter(|m| m.window == window) else { return Continue(()); };
    let Some(key) = menu.pos else { return Continue(()); };
    if &*game.res.facts().get(game.world.block(key.0, key.1, key.2)).name != "tconstruct:part_builder" { return Continue(()); }
    if !g.spectator() && menu.slots.still_valid(&game.world, g, menu.pos) {
        let pattern = usize::try_from(id).ok().and_then(|index| {
            crate::game::tconstruct_source_builder_world::refresh(&mut game.world, key)
                .and_then(|patterns| patterns.get(index).cloned())
        });
        if let Some(pattern) = pattern {
            crate::game::tconstruct_source_builder_world::select(&mut game.world, key, pattern);
        } else if id >= 0 {
            game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().remove(&key);
            game.world.chunks.__horizon_mixins_tconstruct_source_selection_origin_mut().remove(&key);
        }
    }
    Break(())
}
