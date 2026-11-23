// The acting player's item cooldowns, read and started from the game the takeover runs in:
// the host keeps them on the game, and the pure API reaches them through this module while
// the provider call runs. No game pointer escapes: the pointer is set before the provider
// call and cleared when it returns (a guard clears it even if the provider panics), and it
// is only ever used on this thread, one call at a time (the same discipline as the host
// handle in the API crate's hooks).
use std::cell::Cell;

use horizon_client::items::ItemStack;

use super::Game;

/// The game whose cooldowns the current provider call reads, if any.
static CURRENT: Cell<Option<*mut Game<'static>>> = Cell::new(None);

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
pub fn start_cooldown(item: &str, ticks: i32) {
    with_game(|game| {
        let stack = ItemStack::new(item, 1);
        let default = game
            .res
            .items()
            .get(item)
            .and_then(|info| info.use_cooldown.clone());
        let group = horizon_client::cooldowns::group(&stack, default.as_ref());
        game.add_cooldown(group, ticks);
    });
}
