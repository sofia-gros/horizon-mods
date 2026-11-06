// Synchronous primary-local saved knowledge restoration, never a new unlock.
use horizon_client::pluto_inventory::recipe_book::handoff::{
    Handoff, Readiness, Restoration, RestoreError, Saved,
};
use horizon_client_render::chunk_map::pluto_saved_book_state::TakeError as StateError;

struct GameHandoff<'a, 'g>(&'a mut super::Game<'g>);
impl Handoff for GameHandoff<'_, '_> {
    fn restore_saved(
        &mut self,
        owner: &str,
        readiness: Readiness,
        accepts: &mut dyn FnMut(&str) -> bool,
    ) -> Result<Restoration, RestoreError> {
        let game = &mut *self.0;
        if game.remote.is_some() {
            return Err(RestoreError::RemoteClient);
        }
        if game.options.execution_role.is_client() {
            return Err(RestoreError::ClientRole);
        }
        if game.lan_acting.is_some()
            || game.lan_acting_uuid.is_some()
            || game.lan_acting_entity.is_some()
            || game.options.split_guest
        {
            return Err(RestoreError::GuestActor);
        }
        if readiness == Readiness::Defer {
            return Ok(Restoration::Deferred);
        }
        let tables = game
            .world_tables
            .as_ref()
            .ok_or(RestoreError::TablesUnavailable)?;
        // Authority, explicit consumer readiness and native tables precede take.
        // The lock is released before acceptance callbacks or Settings mutation.
        let pending = game
            .world
            .chunks
            .__horizon_mixins_pluto_native_creative_saved_book_pending()
            .take(&game.world_root, owner)
            .map_err(|error| match error {
                StateError::WorldMismatch => RestoreError::WorldMismatch,
                StateError::Poisoned => RestoreError::Poisoned,
            })?;
        let Some((known, highlighted)) = pending else {
            return Ok(Restoration::NoPending);
        };
        let plan = Saved { known, highlighted }
            .plan(|id| tables.stations.recipe_index.contains(id), accepts);
        let book = &mut game.level.recipe_book;
        let recipes_added = plan
            .known
            .into_iter()
            .filter(|id| book.recipes.insert(id.clone()))
            .count();
        let highlights_added = plan
            .highlighted
            .into_iter()
            .filter(|id| book.to_be_displayed.insert(id.clone()))
            .count();
        super::recipe_book::pluto_invalidation::with_game(game, |ui| ui.invalidate_definitions());
        Ok(Restoration::Consumed {
            recipes_added,
            highlights_added,
        })
    }
}
/// No pending data is consumed merely by borrowing this capability.
pub(crate) fn with_game<R>(
    game: &mut super::Game<'_>,
    run: impl FnOnce(&mut dyn Handoff) -> R,
) -> R {
    run(&mut GameHandoff(game))
}
