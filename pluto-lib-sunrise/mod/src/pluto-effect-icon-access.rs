// The local player's active effects' icon flags (vanilla's MobEffectInstance.showIcon),
// published at the head of every HUD draw: the host's effect read carries no icon flag, so
// the game publishes the flags the read is about to need, by namespaced effect id, and the
// API side reads them back when it maps the read. Nothing here knows a HUD, a shift or a
// mod: it is only the last published snapshot.
use std::sync::Mutex;

static ICONS: Mutex<Vec<(String, bool)>> = Mutex::new(Vec::new());

/// Publishes the icon flags, replacing the last snapshot.
pub fn publish(icons: Vec<(String, bool)>) {
    if let Ok(mut slot) = ICONS.lock() { *slot = icons; }
}

/// Whether the effect `id` shows its icon, as the last snapshot published it; `None` when
/// no snapshot names it (no HUD draw yet, or an effect added since).
pub fn show_icon(id: &str) -> Option<bool> {
    ICONS.lock().ok()?.iter().find(|(name, _)| name == id).map(|(_, show)| *show)
}
