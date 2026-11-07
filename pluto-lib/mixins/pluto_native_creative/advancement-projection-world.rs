// Synchronous current-tree projection; never loads or queues a pending tree.
use horizon_client::advancements::pluto_advancement_projection as native;
use horizon_client::pluto_inventory::advancements::projection::{
    Change, Definition, Error, Replacement, TreeSink,
};
use std::sync::Arc;

struct GameTree<'a, 'g>(&'a mut super::Game<'g>);
impl GameTree<'_, '_> {
    fn ready(&self) -> Result<(), Error> {
        if self.0.remote.is_some() {
            return Err(Error::RemoteClient);
        }
        // Empty is the native not-loaded sentinel. This capability cannot add
        // goals and deliberately does not force pending prefetch consumption.
        if self.0.adv.tree.list.is_empty() {
            return Err(Error::NotLoaded);
        }
        Ok(())
    }
}
impl TreeSink for GameTree<'_, '_> {
    fn get(&self, id: &str) -> Result<Option<Definition>, Error> {
        self.ready()?;
        native::get(&self.0.adv.tree, id)
    }
    fn replace(&mut self, changes: &[Replacement]) -> Result<Change, Error> {
        self.ready()?;
        let Some(next) = native::stage(&self.0.adv.tree, changes)? else {
            return Ok(Change::Unchanged);
        };
        self.0.adv.tree = Arc::new(next);
        // Both index paths already compare exact tree Arc identity. Do not
        // touch their ownership, player progress, event hash or pending tree.
        Ok(Change::Changed)
    }
}
pub(crate) fn with_game<R>(
    game: &mut super::Game<'_>,
    run: impl FnOnce(&mut dyn TreeSink) -> R,
) -> R {
    run(&mut GameTree(game))
}
