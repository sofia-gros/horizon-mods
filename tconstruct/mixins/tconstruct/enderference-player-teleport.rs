// ModifierEvents.onTeleport checks the active Enderference effect, regardless
// of amplifier. These two native hooks run with the acting player's state.
fn tconstruct_enderference_player_teleport(
    value: (),
    game: &crate::game::Game<'_>,
    cancel: &mut bool,
) {
    if game.remote.is_none()
        && game
            .potions
            .effects
            .get("tconstruct:enderference")
            .is_some()
    {
        *cancel = true;
    }
    value
}
