// Native max-health attribute publication. Consumer policies choose their own
// transient modifier IDs and values; this module owns no additional state.
use super::Game;

const MAX_HEALTH: &str = "minecraft:max_health";

/// Apply all requested native additive modifiers before clamping health once.
/// This never heals and does not add entries to equipment's reset list. Native
/// per-modifier failures retain their existing effects; this is not a rollback
/// transaction. Projections are values; no consumer callback is accepted.
pub(crate) fn refresh(game: &mut Game<'_>, wanted: &[(&str, Option<f64>)]) {
    if game.remote.is_some()
        || wanted.iter().enumerate().any(|(index, (id, value))|
            id.is_empty() || value.is_some_and(|value| !value.is_finite())
                || wanted[..index].iter().any(|(previous, _)| previous == id)) {
        return;
    }
    let Some(player) = game.player.as_mut() else { return; };
    let mut changed = false;
    for &(id, wanted) in wanted {
        let key = player.attributes.key(MAX_HEALTH);
        let Ok(present) = player.attributes.has_modifier(key.clone(), id) else { continue; };
        if !present && wanted.is_none() { continue; }
        if present && wanted.is_some()
            && player.attributes.modifier_value(key, id).ok() == wanted { continue; }
        if player.set_modifier(MAX_HEALTH, id, wanted.unwrap_or(0.0), "add_value", wanted.is_some()).is_ok() {
            changed = true;
        }
    }
    if changed {
        if let Ok(maximum) = player.attribute(MAX_HEALTH) {
            let maximum = maximum as f32;
            if player.health.health() > maximum {
                let _ = player.set_health(maximum, false);
            }
        }
    }
}
