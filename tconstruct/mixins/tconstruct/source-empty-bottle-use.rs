// Source BottleItem fills the matching water potion from a water source.
fn tconstruct_empty_bottle_use(game: &mut crate::game::Game<'_>, stack: &horizon_items::items::ItemStack) -> std::ops::ControlFlow<horizon_client::Result<bool>> {
    use std::ops::ControlFlow::{Break, Continue};
    let result = match stack.item.as_ref() {
        "tconstruct:splash_bottle" => "minecraft:splash_potion",
        "tconstruct:lingering_bottle" => "minecraft:lingering_potion",
        _ => return Continue(()),
    };
    if game.remote.is_some() { return Continue(()); }
    // The host's ordinary fluid ray skips waterlogged blocks. Its own
    // remote bottle prediction checks the nearer waterlogged outline too.
    let mut chosen = game.pick_fluid();
    if let Some(block_hit) = game.pick() {
        let facts = game.res.facts().get(game.world.block(block_hit.pos[0], block_hit.pos[1], block_hit.pos[2]));
        if facts.waterlogged && facts.fluid == horizon_client::blocks::Fluid::Water && facts.fluid_level == 8 && !facts.fluid_falling {
            let origin = game.aim().map_or(block_hit.point, |(eye, _)| eye);
            let distance = |point: [f64; 3]| (0..3).map(|axis| (point[axis] - origin[axis]).powi(2)).sum::<f64>();
            if chosen.as_ref().is_none_or(|fluid_hit| distance(fluid_hit.point) >= distance(block_hit.point)) { chosen = Some(block_hit); }
        }
    }
    let Some(hit) = chosen else { return Continue(()); };
    if game.res.facts().get(game.world.block(hit.pos[0], hit.pos[1], hit.pos[2])).fluid != horizon_client::blocks::Fluid::Water { return Continue(()); }
    let allowed = match game.lan_acting {
        Some(conn) => game.world.chunks.__horizon_mixins_tconstruct_glow_guest_protection().get(&conn)
            .is_some_and(|policy| crate::game::tconstruct_glowing_placement::outside_protection(*policy, hit.pos)),
        None => game.options.dedicated.is_none(),
    };
    if !allowed || !game.border.contains(f64::from(hit.pos[0]), f64::from(hit.pos[2])) { return Break(Ok(false)); }
    let Some(player) = game.player.as_ref() else { return Break(Ok(false)); };
    let at = [player.position.x, player.position.y, player.position.z];
    game.audio.play("minecraft:item.bottle.fill", Some(at), 1.0, 1.0);
    game.fill_held(ItemStack::new(result, 1).with_potion("minecraft:water"));
    game.post_block_game_event("minecraft:fluid_pickup", hit.pos, game.sculk_actor(), None);
    game.award_stat("used", stack.item.as_ref(), 1);
    Break(Ok(true))
}
