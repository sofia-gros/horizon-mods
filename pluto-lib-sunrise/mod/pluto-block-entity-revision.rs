// Preserve native entity identity and saved-data change invalidation while
// including committed client data/lifetime changes in the opaque cache token.
// This does not touch saved fields, wake server tickers or recurse into revision().
fn pluto_block_entity_revision(entity: &ModBlockEntity) -> ::core::ops::ControlFlow<ModRevision> {
    let mut revision = entity.revision;
    revision.changes = entity.__horizon_mixins_pluto_lib_pluto_block_entity_client_state()
        .cache_token(revision.changes);
    ::core::ops::ControlFlow::Break(revision)
}
