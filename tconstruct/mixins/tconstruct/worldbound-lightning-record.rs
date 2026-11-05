fn tconstruct_worldbound_lightning_record(
    value: Vec<EntityView>,
    game: &crate::game::Game<'_>,
    bolt: &mut crate::game::lightning::Bolt,
) -> Vec<EntityView> {
    crate::game::lightning::tconstruct_worldbound_lightning::record_protected_hits(game, bolt);
    value
}
