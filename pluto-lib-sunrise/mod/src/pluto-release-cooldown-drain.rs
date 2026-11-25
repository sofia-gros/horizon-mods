// After a use's release returns: the drawtime cooldowns the release queued start now,
// after the host's own content cooldown, so the drawtime wins the overwrite.
fn pluto_release_cooldown_drain(value: (), game: &mut Game<'_>) {
    let _ = value;
    crate::game::pluto_item_cooldown::drain_queued(game);
}
