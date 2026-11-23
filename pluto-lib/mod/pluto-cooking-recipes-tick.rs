// The creature tick refreshes the cooking publication whenever the table in use changes,
// and applies the brush strokes a mod ordered at the same tick: the flush in the game's
// brush module applies the queued strokes to its brushable block entities. One injection
// per place: both callbacks run from this single head injection, cooking first.
fn pluto_cooking_recipes_tick(game: &mut crate::game::Game<'_>) {
    crate::entry::pluto_cooking_recipes::refresh(game.recipes());
    crate::game::brush::pluto_brush_flush::flush(game);
}
