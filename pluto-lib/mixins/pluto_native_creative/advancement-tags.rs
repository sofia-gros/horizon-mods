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
            .__horizon_mixins_pluto_native_creative_advancement_tags_mut();
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
            .__horizon_mixins_pluto_native_creative_advancement_tags_mut()
            .cache = None;
    }
}
pub(crate) fn with_game<R>(
    game: &mut super::Game<'_>,
    run: impl FnOnce(&mut dyn TagSink) -> R,
) -> R {
    run(&mut GameTags(game))
}
