// Use the existing authoritative player break path for each source AOE block.
fn tconstruct_area_break(game: &mut crate::game::Game<'_>, pos: [i32;3]) -> std::ops::ControlFlow<Result<()>> {
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() { return Continue(()); }
    let actor = game.lan_acting;
    if game.world.chunks.__horizon_mixins_tconstruct_area_active().contains(&actor) { return Continue(()); }
    let effective = |game: &crate::game::Game<'_>, block: &str| -> bool {
        let Some(stack) = game.held() else { return false; };
        if !matches!(stack.item.as_ref(), "tconstruct:sledge_hammer" | "tconstruct:excavator") { return false; }
        let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return false; };
        let Some(Tag::Int(max)) = custom.get("tic_max_damage") else { return false; };
        if *max <= 0 || stack.damage >= *max as u32 { return false; }
        let Some(tool) = custom.get("tic_mining_tool") else { return false; };
        crate::game::tconstruct_source_mining_rules::harvestable(tool, block, |tag, block| game.res.items().block_in_tag(tag, block)) == Some(true)
    };
    let state = game.world.block(pos[0], pos[1], pos[2]);
    let facts = game.res.facts().get(state);
    let hardness = facts.hardness;
    if hardness < 0.0 || !effective(game, &facts.name) { return Continue(()); }
    let face = match actor {
        Some(conn) => game.world.chunks.__horizon_mixins_tconstruct_area_guest_hits().get(&conn)
            .filter(|(target, _)| *target == pos).map(|(_, face)| *face),
        None => game.pick().filter(|hit| hit.pos == pos).map(|hit| hit.face),
    };
    let Some(positions) = face.and_then(|face| crate::game::tconstruct_source_area_shape::plane(pos, face, game.yaw)) else { return Continue(()); };
    game.world.chunks.__horizon_mixins_tconstruct_area_active_mut().insert(actor);
    let result = (|| -> Result<()> {
        game.break_block_as_player(pos)?;
        if game.world.block(pos[0], pos[1], pos[2]) == state { return Ok(()); }
        for extra in positions {
            let state = game.world.block(extra[0], extra[1], extra[2]);
            if game.res.facts().is_air(state) { continue; }
            let facts = game.res.facts().get(state);
            if !crate::game::tconstruct_source_area_shape::compatible_hardness(hardness, facts.hardness) || !effective(game, &facts.name) { continue; }
            game.break_block_as_player(extra)?;
        }
        Ok(())
    })();
    game.world.chunks.__horizon_mixins_tconstruct_area_active_mut().remove(&actor);
    if let Some(conn) = actor { game.world.chunks.__horizon_mixins_tconstruct_area_guest_hits_mut().remove(&conn); }
    Break(result)
}
