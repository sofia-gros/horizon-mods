// After the entity interaction returns: the chestplate takeover runs when the game did not consume.
fn pluto_sleeves_takeover_entity_return(value: Result<bool>, game: &mut Game<'_>) {
    crate::game::item_actions::pluto_sleeves_takeover_entity::after_entity_use(game, value);
}
