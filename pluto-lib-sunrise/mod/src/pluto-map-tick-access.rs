// The shared state of PlutoLib's stored-map tick: the provider the API crate installs, and the
// offhand swap a map tick is running. The game-side module in `game/maps.rs` asks the provider
// for the worn head's stored map id, swaps a filled map with that id into the offhand for the
// vanilla map tick, and restores the old offhand stack when the tick returns. Nothing here
// knows a helmet, a minimap or a mod.
use std::sync::Mutex;

use sunrise_items::items::ItemStack;
use mod_api::Host;

/// The offhand swap a map tick is running: the stack the tick displaced, the id of the map
/// ticking in its place, and the holder the update packet goes to (`HOST` for the local player,
/// the guest index for a LAN guest).
#[derive(Clone)]
pub struct Swap {
    pub old_offhand: Option<ItemStack>,
    pub map_id: i32,
    pub who: u32,
}

/// What the API crate provides: whether a map-tick handler listens, and the callback that
/// answers a worn head stack's stored map id.
#[derive(Clone, Copy)]
pub struct Provider {
    pub enabled: fn() -> bool,
    pub map_tick: fn(&mut dyn Host, &ItemStack) -> Option<i32>,
}

static PROVIDER: Mutex<Option<Provider>> = Mutex::new(None);

/// Installs the provider. Called once by the API crate's hook installation.
pub fn set_provider(provider: Provider) {
    if let Ok(mut slot) = PROVIDER.lock() {
        *slot = Some(provider);
    }
}

/// The installed provider, if any.
pub fn provider() -> Option<Provider> {
    PROVIDER.lock().ok().and_then(|slot| slot.map(|provider| Provider { ..provider }))
}

/// Whether a map-tick handler listens, so the game only reads the head stack when someone does.
pub fn enabled() -> bool {
    provider().map_or(false, |provider| (provider.enabled)())
}

/// Answers a worn head stack's stored map id through the installed provider; `None` when none
/// is installed, or when no handler answers.
pub fn on_map_tick_head(host: &mut dyn Host, head: &ItemStack) -> Option<i32> {
    let Some(provider) = provider() else { return None };
    (provider.map_tick)(host, head)
}

static SWAP: Mutex<Option<Swap>> = Mutex::new(None);

/// Stashes the offhand swap a map tick is running, so the return half restores it.
pub fn stash(swap: Swap) {
    if let Ok(mut slot) = SWAP.lock() {
        *slot = Some(swap);
    }
}

/// Takes the stashed offhand swap, if any, so a stale swap is never restored twice.
pub fn take_swap() -> Option<Swap> {
    SWAP.lock().ok().and_then(|mut slot| slot.take())
}
