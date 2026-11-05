fn tconstruct_looting_death_head(
    game: &mut crate::game::Game<'_>,
    victim: &horizon_client::mobs::Mob,
    player_uuid: Option<u128>,
    by_player: bool,
    killer: &mut horizon_world::loot::LootEntity,
) {
    self::tconstruct_looting_death_world::apply(game, victim, player_uuid, by_player, killer);
}
