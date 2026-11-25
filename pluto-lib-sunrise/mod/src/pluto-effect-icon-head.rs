// Before the HUD draws: the local player's active effects publish their icon flags.
fn pluto_effect_icon_head(game: &mut crate::game::Game<'_>) {
    game.publish_effect_icons();
}
