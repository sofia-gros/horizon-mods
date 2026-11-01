// The source tick advancement grants its loot once per player. The native
// impossible criterion lets the config-controlled hook award it, while the
// existing player advancement progress owns persistence and the loot reward.
fn tconstruct_starting_book(game: &mut Game<'_>) {
    if game.player.is_none() || game.remote.is_some() { return; }
    let Some(config) = game.world.chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref().and_then(|v| v.as_ref().ok()) else { return; };
    if !config.get("spawn_with_book").copied().unwrap_or(false) { return; }
    game.load_advancements();
    let Some(index) = game.adv.tree.by_id.get("tconstruct:internal/starting_book").copied() else { return; };
    game.grant_advancement(index, None);
}
