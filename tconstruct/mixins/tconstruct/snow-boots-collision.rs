fn tconstruct_snow_boots_collision(
    mut value: Option<crate::game::PlayerCollision>,
    game: &crate::game::Game<'_>,
) -> Option<crate::game::PlayerCollision> {
    if let Some(collision) = value.as_mut() {
        collision.leather_boots = crate::game::tconstruct_source_snow_boots::walks(game);
    }
    value
}
