// Native presentation-cache adapter, owned only by PlutoLib. No recipe policy.
// Installed below game::recipe_book, where the host's private UI fields live.
use super::Game;
use horizon_client::pluto_inventory::recipe_book::Invalidation;

struct GameInvalidation<'a, 'g>(&'a mut Game<'g>);
impl Invalidation for GameInvalidation<'_, '_> {
    fn invalidate_definitions(&mut self) {
        let game = &mut *self.0;
        // The native operation clears both the book model and category cache.
        game.forget_book();
        game.recipe_ui.client_known = None;
        game.recipe_ui.view = Default::default();
        game.recipe_ui.ghost.clear();
        game.recipe_ui.overlay = None;
        game.recipe_ui.last_placed = None;
        game.recipe_ui.last_clicked = None;
    }
}

/// Borrow only for this call. No registry, actor retention or automatic trigger.
/// The caller owns timing/remote policy; this touches presentation caches only.
pub(crate) fn with_game<R>(game: &mut Game<'_>, run: impl FnOnce(&mut dyn Invalidation) -> R) -> R {
    run(&mut GameInvalidation(game))
}
