fn tconstruct_side_parts(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<Vec<((i32, i32, i32), usize)>> {
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() || crate::game::tconstruct_ui::kind(game) != Some(false) { return Continue(()); }
    let Some(key) = game.container else { return Continue(()); };
    let Some(&side) = game.world.chunks.__horizon_mixins_tconstruct_builder_side().get(&key) else { return Continue(()); };
    let Some(entity) = game.world.block_entities.get(&side) else { return Continue(()); };
    let mut parts = vec![(key, 27), (side, entity.slots().len())];
    if let Some((partner, _)) = game.chest_partner(side) {
        if let Some(entity) = game.world.block_entities.get(&partner) { parts.push((partner, entity.slots().len())); }
    }
    Break(parts)
}
