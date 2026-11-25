use horizon_client::pluto_inventory::advancements::tags::{
    self, Change, ReplaceError, TagSink, TagSnapshot,
};
struct GameTags<'a, 'g>(&'a mut super::Game<'g>);
impl TagSink for GameTags<'_, '_> {
    fn replace_owner(&mut self, owner: &str, tags: TagSnapshot) -> Result<Change, ReplaceError> {
        let state = self
            .0
            .world
            .chunks
            .__horizon_mixins_pluto_lib_pluto_advancement_tags_mut();
        let mut next = (*state.owners).clone();
        let change = tags::replace_owner(&mut next, owner, tags)?;
        if change == Change::Changed {
            state.owners = std::sync::Arc::new(next);
            state.cache = None;
        }
        Ok(change)
    }
    fn invalidate_candidates(&mut self) {
        self.0
            .world
            .chunks
            .__horizon_mixins_pluto_lib_pluto_advancement_tags_mut()
            .cache = None;
    }
}
pub(crate) fn with_game<R>(
    game: &mut super::Game<'_>,
    run: impl FnOnce(&mut dyn TagSink) -> R,
) -> R {
    run(&mut GameTags(game))
}

/// The TagSink owner driver: apply every owner snapshot the installed tag
/// source currently publishes to this world's store, through the sink
/// above. Runs at the head of the advancement tick (see
/// pluto-advancement-event-drain), before queued events drain, so a tick's
/// evaluations see the snapshots that were current when the tick began.
/// The source's publication generation is compared against the generation
/// last applied to this store, so an unchanged publication costs one
/// comparison per tick; a fresh or cleared store (applied == 0) is always
/// driven. A rejected owner (a conflict with another owner's tags, an
/// empty owner) is reported and skipped: replace_owner is atomic, so the
/// store keeps the last good snapshot for that owner.
pub(crate) fn drive(game: &mut super::Game<'_>) {
    let generation = super::pluto_advancement_tag_source::generation();
    if generation == 0 {
        return;
    }
    if game
        .world
        .chunks
        .__horizon_mixins_pluto_lib_pluto_advancement_tags()
        .applied
        == generation
    {
        return;
    }
    let snapshots = super::pluto_advancement_tag_source::take_snapshots();
    with_game(game, |sink| {
        for snapshot in snapshots {
            if let Err(error) = sink.replace_owner(&snapshot.owner, snapshot.tags) {
                eprintln!("pluto_lib: advancement tag snapshot rejected: {error:?}");
            }
        }
    });
    game.world
        .chunks
        .__horizon_mixins_pluto_lib_pluto_advancement_tags_mut()
        .applied = generation;
}
