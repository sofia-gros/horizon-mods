// The shared state of PlutoLib's fishing reach: the provider the API crate installs (its queue of rod
// uses and its wear reports live in the pure fishing API), the combat hook currently out, and the hit
// it last took. The game-side module in `game/fishing.rs` works through this; nothing here knows a
// rod, a profile's meaning or a mod.

use std::sync::Mutex;

/// A rod's cast numbers, carried plainly (the pure profile, field for field).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UseProfile {
    pub luck: i32,
    pub lure: i32,
    pub velocity: f32,
    pub inaccuracy: f32,
    pub power: f32,
    pub knockback: f32,
    /// 0: no grapple, 1: dash, 2: drill.
    pub grapple: u8,
    pub collecting: bool,
}

/// The combat hook currently out: whose use cast it, with what profile, and its entity id (the game's
/// hook carries no owner or kind of its own, so the reach remembers which hook is the combat one).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Active {
    pub uuid: u128,
    pub profile: UseProfile,
    pub hook_id: u32,
}

/// A hit the combat hook took: the entity it hooked and the speed it struck at (the hook's own
/// velocity is zeroed by the hit, so the speed is captured before it lands).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub target: u32,
    pub speed: f32,
}

/// What the API crate provides: the queued rod uses, the wear sink, and the rod names.
#[derive(Clone, Copy)]
pub struct Provider {
    pub take_use: fn() -> Option<(u128, UseProfile)>,
    pub note_wear: fn(u128, u32),
    pub is_rod: fn(&str) -> bool,
}

static PROVIDER: Mutex<Option<Provider>> = Mutex::new(None);
static ACTIVE: Mutex<Option<Active>> = Mutex::new(None);
static PENDING_HIT: Mutex<Option<Hit>> = Mutex::new(None);

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

/// The combat hook currently out, if any.
pub fn active() -> Option<Active> {
    ACTIVE.lock().ok().and_then(|slot| *slot)
}

/// Remembers the combat hook that was just cast.
pub fn set_active(active: Active) {
    if let Ok(mut slot) = ACTIVE.lock() {
        *slot = Some(active);
    }
}

/// Forgets the combat hook (it was retrieved, or it ended another way).
pub fn clear_active() {
    if let Ok(mut slot) = ACTIVE.lock() {
        *slot = None;
    }
    if let Ok(mut hit) = PENDING_HIT.lock() {
        *hit = None;
    }
}

/// Records the hit the combat hook took.
pub fn note_hit(target: u32, speed: f32) {
    if let Ok(mut hit) = PENDING_HIT.lock() {
        *hit = Some(Hit { target, speed });
    }
}

/// The hit the combat hook took, if any, drained.
pub fn take_hit() -> Option<Hit> {
    PENDING_HIT.lock().ok()?.take()
}
