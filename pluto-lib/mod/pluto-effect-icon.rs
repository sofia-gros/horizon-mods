// At the head of the HUD draw: publish the local player's active effects' icon flags
// (vanilla's MobEffectInstance.showIcon), so the effect read can carry the flag the host's
// own effect read does not. The HUD shift counts exactly the effects that show their icon
// (ModifierClientEvents.getEffectOffset); the snapshot is the local player's own list, read
// where the draw is about to run, so it names the effects the draw is about to be given.
use super::Game;

impl Game<'_> {
    /// Publishes the local player's active effects' icon flags, by namespaced effect id, for
    /// the effect read: an effect the snapshot does not name shows its icon, as vanilla
    /// defaults the flag.
    pub(crate) fn publish_effect_icons(&mut self) {
        let icons = self.potions.effects.list.iter().map(|effect| (namespaced(effect.id), effect.show_icon)).collect();
        crate::entry::pluto_effect_icon_access::publish(icons);
    }
}

/// `minecraft:` in front of an id without a namespace, as the host's own
/// `native_player_state::namespaced` (and `brewing::namespaced`) spells the effect ids the
/// host's effect read reports.
fn namespaced(id: &str) -> String {
    if id.contains(':') { id.to_owned() } else { format!("minecraft:{id}") }
}
