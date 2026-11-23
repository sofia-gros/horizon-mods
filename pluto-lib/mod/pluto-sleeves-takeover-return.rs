// After the hand's use returns: the chestplate takeover runs when the use did not consume.
fn pluto_sleeves_takeover_return(value: Result<bool>, game: &mut Game<'_>) {
    crate::game::use_hand::pluto_sleeves_takeover::after_use(game, value);
}
