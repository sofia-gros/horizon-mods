// The creature tick refreshes the cooking publication whenever the table in use changes,
// applies the brush strokes a mod ordered, and ends the item uses a mod ordered ended:
// the flushes in the game's brush and using_item modules apply what was queued to the
// level and the use in place. One injection per place: all three callbacks run from this
// single head injection, cooking first, and a use's strokes before its end.
fn pluto_cooking_recipes_tick(game: &mut crate::game::Game<'_>) {
    crate::entry::pluto_cooking_recipes::refresh(game.recipes());
    crate::game::brush::pluto_brush_flush::flush(game);
    crate::game::using_item::pluto_use_stop_flush::flush(game);
}
