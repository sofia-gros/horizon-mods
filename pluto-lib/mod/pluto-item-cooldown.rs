// The acting player's item cooldowns, read and started from the game the takeover runs in:
// the host keeps them on the game, and the pure API reaches them through this module while
// the provider call runs. No game pointer escapes: the pointer is set before the provider
// call and cleared when it returns (a guard clears it even if the provider panics), and it
// is only ever used on this thread, one call at a time (the same discipline as the host
// handle in the API crate's hooks).
//
// A release's dispatch holds no game for its call, so a start asked for there queues
// instead of starting at once; the release drain (the RETURN on
// Game::mod_release_using) applies the queued starts after the host's own content
// cooldown, so the drawtime a mod asked for wins the overwrite.
use std::cell::Cell;

use horizon_client::items::ItemStack;

use super::Game;

/// The game whose cooldowns the current provider call reads, if any.
static CURRENT: Cell<Option<*mut Game<'static>>> = Cell::new(None);

/// Cooldowns a release asked to start while no game was held for its call, as
/// (item id, ticks), in the order they were asked for. The release drain
/// applies them after the dispatch returns.
static QUEUED: Cell<Vec<(String, i32)>> = Cell::new(Vec::new());

/// Holds the game for the provider call: set on creation, cleared on drop, so a panic in the
/// provider cannot leave a stale pointer behind.
pub struct CurrentGuard;

impl Drop for CurrentGuard {
    fn drop(&mut self) {
        CURRENT.set(None);
    }
}

/// Sets the game whose cooldowns the current provider call reads, returning a guard that
/// clears it when the call returns.
pub fn hold(game: &mut Game<'_>) -> CurrentGuard {
    CURRENT.set(Some(game as *mut Game<'static>));
    CurrentGuard
}

/// Runs `f` over the game the current provider call holds, if any.
pub fn with_game<R>(f: impl FnOnce(&mut Game<'_>) -> R) -> Option<R> {
    let game = CURRENT.get()?;
    // SAFETY: the pointer is the game the takeover holds for this provider call, set by
    // `hold` and cleared by its guard when the call returns; it is only used on this thread,
    // and the takeover does not touch the game while the provider runs.
    Some(f(unsafe { &mut *game }))
}

/// Whether `item` is on the acting player's cooldown (the item's own `use_cooldown`
/// component, else the item's default; false outside a provider call or for an item with no
/// cooldown).
pub fn is_on_cooldown(item: &str) -> bool {
    with_game(|game| {
        let stack = ItemStack::new(item, 1);
        let default = game
            .res
            .items()
            .get(item)
            .and_then(|info| info.use_cooldown.clone());
        let group = horizon_client::cooldowns::group(&stack, default.as_ref());
        game.cooldowns.is_on_cooldown(&group)
    })
    .unwrap_or(false)
}

/// Starts the acting player's cooldown for `item` for `ticks` ticks (the item's own
/// `use_cooldown` component, else the item's default, names the group; nothing happens
/// outside a provider call or for an item with no cooldown). The game's own add runs, so a
/// guest also gets the cooldown packet.
///
/// A provider call holds the game, so the cooldown starts at once (the sleeves throw
/// gates on it). A release's dispatch holds no game, so the start queues for the release
/// drain instead (see `queue_cooldown`).
pub fn start_cooldown(item: &str, ticks: i32) {
    if with_game(|game| {
        let stack = ItemStack::new(item, 1);
        let default = game
            .res
            .items()
            .get(item)
            .and_then(|info| info.use_cooldown.clone());
        let group = horizon_client::cooldowns::group(&stack, default.as_ref());
        game.add_cooldown(group, ticks);
    })
    .is_none()
    {
        queue_cooldown(item, ticks);
    }
}

/// Queues a start for the release drain: the item's own `use_cooldown` component, else the
/// item's default, names the group when the drain runs. The drain applies the queue in
/// order, so a start asked for during a release lands after the host's own content
/// cooldown for that release.
pub fn queue_cooldown(item: &str, ticks: i32) {
    let mut queued = QUEUED.take();
    queued.push((item.to_owned(), ticks));
    QUEUED.set(queued);
}

/// Starts every queued cooldown on the acting player, in the order they were asked for.
/// The release drain runs this after the host's own content cooldown, so a queued
/// drawtime overwrites it. A guest gets the packet through the game's own add, as a
/// synchronous start does.
pub fn drain_queued(game: &mut Game<'_>) {
    for (item, ticks) in QUEUED.take() {
        let stack = ItemStack::new(&item, 1);
        let default = game
            .res
            .items()
            .get(&item)
            .and_then(|info| info.use_cooldown.clone());
        let group = horizon_client::cooldowns::group(&stack, default.as_ref());
        game.add_cooldown(group, ticks);
    }
}
