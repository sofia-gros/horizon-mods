// Candidate only: one compute-only consumer at the official BreakSpeed HEAD.
// The Pluto private adapter borrows native facts; TConstruct retains all policy.
// Keep original untouched and let the official mod dispatch run afterward.
fn tconstruct_material_mining_context_native(
    game: &crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
    speed: &mut f32,
) {
    let context = crate::game::pluto_mining_context::read(game, pos, state);
    *speed = crate::game::tconstruct_source_mining_context::apply(
        *speed,
        game,
        &context.facts,
        &context.held,
        Some((context.pos[1], context.max_y)),
    );
}
