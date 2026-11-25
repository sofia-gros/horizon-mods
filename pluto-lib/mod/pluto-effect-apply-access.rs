// The icon flag (vanilla's MobEffectInstance.showIcon) of the one effect the API side is
// giving a player now, recorded for the add it belongs to: the host's add path
// (NativeEffect) carries no icon flag and defaults it to shown, so the API side records
// the flag it was given and the game takes it as the effect is added (see
// pluto-effect-apply-head). A record an add never takes (an unknown effect, a player
// the host cannot reach) is dropped by the API side's clear, so it can never land on a
// later, unrelated add of the same effect.
use std::sync::Mutex;

static PENDING: Mutex<Option<(String, bool)>> = Mutex::new(None);

/// Records the icon flag of the effect `id` the API side is about to give, for the add
/// that immediately follows; a record already pending is replaced.
pub fn request(id: String, show_icon: bool) {
    if let Ok(mut slot) = PENDING.lock() { *slot = Some((id, show_icon)); }
}

/// Takes the recorded icon flag for the effect `id`, the host's own raw id, when the
/// pending record names that effect (the record is keyed by the namespaced id the API
/// side gives, the spelling the host's effect read reports); a record naming another
/// effect is left alone.
pub fn take(id: &str) -> Option<bool> {
    let mut slot = PENDING.lock().ok()?;
    let key = namespaced(id);
    let named = matches!(slot.as_ref(), Some((name, _)) if name == &key || name.as_str() == id);
    if named { slot.take().map(|(_, show_icon)| show_icon) } else { None }
}

/// Drops a recorded flag no add took, so it cannot land on a later add.
pub fn clear() {
    if let Ok(mut slot) = PENDING.lock() { *slot = None; }
}

/// `minecraft:` in front of an id without a namespace, as the host's own
/// `native_player_state::namespaced` (and `brewing::namespaced`) spells the effect ids
/// the API side gives.
fn namespaced(id: &str) -> String {
    if id.contains(':') { id.to_owned() } else { format!("minecraft:{id}") }
}
