//! ConductingModifier.bonusScale (SlimeKnights, MIT; see LICENSE): how much of a fire-scaled
//! bonus an entity earns from burning — nothing while not on fire, the share of 200 ticks of
//! fire it has left (the full bonus at 200), halved while it resists fire. Shared by the
//! conducting modifier itself and by FlameBarrierModule, which reads the same scale.
use pluto_lib::EntityState;

/// The fire ticks at which the bonus is whole (ConductingModifier.MAX_BONUS_TICKS).
const MAX_BONUS_TICKS: i32 = 200;

/// The scale in [0, 1] for the entity state PlutoLib reported; 0 when it reported none.
pub(crate) fn bonus_scale(state: Option<&EntityState>) -> f32 {
    let Some(state) = state else { return 0.0 };
    if state.fire_ticks <= 0 { return 0.0; }
    let scale = state.fire_ticks.min(MAX_BONUS_TICKS) as f32 / MAX_BONUS_TICKS as f32;
    if state.effects.iter().any(|effect| effect == "minecraft:fire_resistance") { scale * 0.5 } else { scale }
}

/// The percent bonus a level of the conducting modifier is worth (ConductingModifier.PERCENT_PER_LEVEL).
pub(crate) const PERCENT_PER_LEVEL: f32 = 0.2;

/// ConductingModifier.getMeleeDamage and modifyStat: the factor a conducting entry of
/// `effective_level` multiplies a damage value by for an attacker (or holder) in `state`.
pub(crate) fn damage_multiplier(state: Option<&EntityState>, effective_level: f32) -> f32 {
    1.0 + bonus_scale(state) * effective_level * PERCENT_PER_LEVEL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_follows_the_fire_that_is_left() {
        let state = |fire_ticks: i32, effects: Vec<String>| EntityState { fire_ticks, effects, ..EntityState::default() };
        assert_eq!(bonus_scale(None), 0.0);
        assert_eq!(bonus_scale(Some(&state(0, vec![]))), 0.0);
        assert_eq!(bonus_scale(Some(&state(-5, vec![]))), 0.0);
        assert_eq!(bonus_scale(Some(&state(100, vec![]))), 0.5);
        assert_eq!(bonus_scale(Some(&state(200, vec![]))), 1.0);
        assert_eq!(bonus_scale(Some(&state(600, vec![]))), 1.0, "capped at the 200 tick bonus");
        assert_eq!(bonus_scale(Some(&state(200, vec!["minecraft:fire_resistance".into()]))), 0.5, "fire resistance halves it");
    }

    #[test]
    fn the_damage_multiplier_adds_twenty_percent_a_level_at_full_fire() {
        let burning = EntityState { fire_ticks: 200, ..EntityState::default() };
        assert_eq!(damage_multiplier(Some(&burning), 2.0), 1.4);
        assert_eq!(damage_multiplier(Some(&burning), 0.0), 1.0);
        assert_eq!(damage_multiplier(None, 2.0), 1.0);
    }
}
