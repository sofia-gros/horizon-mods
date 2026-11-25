// The shared state of PlutoLib's sleeves takeover: the provider the API crate installs, and the
// use the game's hand just finished. The game-side modules in `game/use_hand.rs` and
// `game/item_actions.rs` report the use; nothing here knows a sleeve, a cooldown or a mod.
use std::sync::Mutex;

use horizon_client::mod_api::Host;

/// What the hand's use left behind: who used it, where they aimed, and whether the game
/// consumed it. The provider reads the aim (the eye and the look the throw aims by) and the
/// crouch state the sleeves module branches on; `consumed` is false at both seams that
/// report a use today (the hand's use, and a click on an entity the game did not take; the
/// takeover runs only when the game did not consume), and stays so a later seam can report
/// a consumed use the same way.
#[derive(Clone, Debug, PartialEq)]
pub struct PostUse {
    pub uuid: u128,
    pub name: String,
    /// The dimension, such as `minecraft:overworld`.
    pub dimension: String,
    /// Where the player's eyes are, in block coordinates.
    pub eye: [f64; 3],
    /// Where the player looks, a unit vector.
    pub look: [f64; 3],
    /// Whether the player is sneaking.
    pub crouching: bool,
    /// Whether the player is in creative mode (nothing is used up).
    pub creative: bool,
    /// Whether the game's own handling consumed the use.
    pub consumed: bool,
}

/// What the API crate provides: the callback that answers a finished use. True when a
/// callback took the use.
#[derive(Clone, Copy)]
pub struct Provider {
    pub post_use: fn(&mut dyn Host, &PostUse) -> bool,
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

/// Whether a provider is installed, so the game only builds the use when someone listens.
pub fn active() -> bool {
    PROVIDER.lock().ok().map_or(false, |slot| slot.is_some())
}

/// Answers a finished use through the installed provider; false when none is installed.
pub fn on_post_use(host: &mut dyn Host, post: &PostUse) -> bool {
    let Some(provider) = provider() else { return false };
    (provider.post_use)(host, post)
}
