// The acting player's item cooldowns, read and started from the game the takeover runs in:
// the host keeps them on the game, and the pure API reaches them through this module while
// the provider call runs. No game pointer is kept: the call reads a snapshot of the game's
// cooldown state taken when the call began (the entries as they stood, and the item
// defaults the group lookups need), and the starts it asks for land on the game when the
// call returns — the takeover settles them before the use goes on, inside the same turn,
// so the game the world sees carries them exactly as if they had started when asked for.
// The snapshot lives on this thread only, one call at a time, and a guard clears it when
// the call returns (even if the provider panics; a panicked call's unapplied starts go
// with it, as its other unfinished work does).
//
// A release's dispatch holds no game for its call, so a start asked for there queues
// instead of starting at once; the release drain (the RETURN on
// Game::mod_release_using) applies the queued starts after the host's own content
// cooldown, so the drawtime a mod asked for wins the overwrite.
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use sunrise_items::cooldowns::{Cooldowns, UseCooldown};
use sunrise_items::items::ItemStack;

use super::Game;

thread_local! {
    /// The cooldown state of the game the current provider call runs in, snapshotted
    /// when the call began; empty outside a provider call.
    static CURRENT: RefCell<Option<Held>> = const { RefCell::new(None) };
}

/// Cooldowns a release asked to start while no game was held for its call, as
/// (item id, ticks), in the order they were asked for. The release drain
/// applies them after the dispatch returns. The queue is shared rather than
/// thread-local: a release's dispatch and its drain need not run on one
/// thread, and the lock keeps the order they were asked for either way.
static QUEUED: LazyLock<Mutex<Vec<(String, i32)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// The held cooldown state for one provider call: the game's cooldown entries
/// as they stood when the call began, the registry defaults its group lookups
/// need (only the items that carry one; every other item's default is none,
/// exactly as the registry answers for it), and the starts the call has asked
/// for so far, in the order it asked for them.
struct Held {
    cooldowns: Cooldowns,
    defaults: HashMap<Arc<str>, UseCooldown>,
    started: Vec<(Arc<str>, i32)>,
}

/// The cooldown group `item` files under in the held state: the host's own
/// `group` over a fresh stack, with the item's default from the snapshot.
fn held_group(held: &Held, item: &str) -> Arc<str> {
    let stack = ItemStack::new(item, 1);
    sunrise_items::cooldowns::group(&stack, held.defaults.get(item))
}

/// Applies the held call's starts to `game`, in the order they were asked
/// for, through the game's own add (so a guest also gets the packet).
fn apply_started(game: &mut Game<'_>) {
    let started = CURRENT.with(|slot| {
        slot.borrow_mut()
            .as_mut()
            .map(|held| std::mem::take(&mut held.started))
            .unwrap_or_default()
    });
    for (group, ticks) in started {
        game.add_cooldown(group, ticks);
    }
}

/// Holds the game's cooldown state for the provider call: snapshotted on
/// creation, cleared on drop, so a panic in the provider cannot leave a
/// stale state behind.
pub struct CurrentGuard;

impl CurrentGuard {
    /// Settles the call: the starts it asked for land on the game it ran in,
    /// in the order they were asked for, before the use goes on — so each
    /// cooldown is the game's own from the moment the call returns, exactly
    /// as if it had started when asked for.
    pub fn settle(self, game: &mut Game<'_>) {
        apply_started(game);
    }
}

impl Drop for CurrentGuard {
    fn drop(&mut self) {
        CURRENT.with(|slot| slot.borrow_mut().take());
    }
}

/// Sets the cooldown state the current provider call reads, returning a guard
/// that clears it when the call returns. A use the provider's answer set off
/// is entered fresh inside the outer call: the outer call's starts land on
/// the game first (they were asked for first, and the game the nested call
/// snapshots must already carry them), then the nested call snapshots the
/// game as it now stands.
pub fn hold(game: &mut Game<'_>) -> CurrentGuard {
    apply_started(game);
    let held = Held {
        cooldowns: game.cooldowns.clone(),
        defaults: game
            .res
            .items()
            .items
            .iter()
            .filter_map(|(id, info)| {
                info.use_cooldown
                    .clone()
                    .map(|default| (Arc::clone(id), default))
            })
            .collect(),
        started: Vec::new(),
    };
    CURRENT.with(|slot| *slot.borrow_mut() = Some(held));
    CurrentGuard
}

/// Whether `item` is on the acting player's cooldown (the item's own `use_cooldown`
/// component, else the item's default; false outside a provider call or for an item with no
/// cooldown).
pub fn is_on_cooldown(item: &str) -> bool {
    CURRENT.with(|slot| {
        let slot = slot.borrow();
        let Some(held) = slot.as_ref() else {
            return false;
        };
        held.cooldowns.is_on_cooldown(&held_group(held, item))
    })
}

/// Starts the acting player's cooldown for `item` for `ticks` ticks (the item's own
/// `use_cooldown` component, else the item's default, names the group; nothing happens
/// outside a provider call or for an item with no cooldown). The game's own add runs
/// when the call settles, so a guest also gets the cooldown packet.
///
/// A provider call holds the game's cooldown state, so the start counts at once for the
/// call's own reads (the sleeves throw gates on it) and lands on the game when the call
/// settles. A release's dispatch holds no game, so the start queues for the release
/// drain instead (see `queue_cooldown`).
pub fn start_cooldown(item: &str, ticks: i32) {
    let held_call = CURRENT.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(held) = slot.as_mut() else {
            return false;
        };
        let group = held_group(held, item);
        held.cooldowns.add(Arc::clone(&group), ticks);
        held.started.push((group, ticks));
        true
    });
    if !held_call {
        queue_cooldown(item, ticks);
    }
}

/// Queues a start for the release drain: the item's own `use_cooldown` component, else the
/// item's default, names the group when the drain runs. The drain applies the queue in
/// order, so a start asked for during a release lands after the host's own content
/// cooldown for that release.
pub fn queue_cooldown(item: &str, ticks: i32) {
    QUEUED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push((item.to_owned(), ticks));
}

/// Starts every queued cooldown on the acting player, in the order they were asked for.
/// The release drain runs this after the host's own content cooldown, so a queued
/// drawtime overwrites it. A guest gets the packet through the game's own add, as a
/// synchronous start does.
pub fn drain_queued(game: &mut Game<'_>) {
    let queued = std::mem::take(
        &mut *QUEUED
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    );
    for (item, ticks) in queued {
        let stack = ItemStack::new(&item, 1);
        let default = game
            .res
            .items()
            .get(&item)
            .and_then(|info| info.use_cooldown.clone());
        let group = sunrise_items::cooldowns::group(&stack, default.as_ref());
        game.add_cooldown(group, ticks);
    }
}
