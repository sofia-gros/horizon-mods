// Capture a successfully prepared world at open entry, before native filtering.
// This does not claim that finish_open_world has completed successfully yet.
use horizon_client::pluto_inventory::recipe_book::handoff;

pub(crate) fn capture(prepared: &super::world_loading::PreparedWorld) {
    if !handoff::enabled() {
        return;
    }
    let book = &prepared.level.recipe_book;
    // Every selector runs before acquiring the pending-state lock.
    let selected = match handoff::capture(&book.recipes, &book.to_be_displayed) {
        Ok(selected) => selected,
        Err(error) => {
            log::warn!("PlutoLib saved-book capture rejected: {error:?}");
            return;
        }
    };
    let owners = selected
        .into_iter()
        .map(|(owner, saved)| (owner, (saved.known, saved.highlighted)))
        .collect();
    if let Err(error) = prepared
        .world
        .chunks
        .__horizon_mixins_pluto_native_creative_saved_book_pending()
        .install(prepared.save_root.clone(), owners)
    {
        log::warn!("PlutoLib saved-book handoff unavailable: {error:?}");
    }
}
