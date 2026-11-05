/// Conducting is a statically registered Java modifier. A dynamic definition
/// at the same ID is not assumed to retain its original hooks.
pub(super) fn supported_definition(world: &super::tconstruct_world::World) -> bool {
    world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok())
        .is_some_and(|raw| !raw.0.contains_key("tconstruct:tinkering/modifiers/conducting"))
}

/// ConductingModifier.bonusScale uses remaining fire, not visible on-fire state.
/// The cap is 200 ticks, despite its stale "15 seconds" comment.
pub(super) fn factor(fire_ticks: i32, fire_resistance: bool, level: i32) -> Option<f32> {
    if level <= 0 { return None; }
    let mut bonus = if fire_ticks <= 0 { 0.0 }
        else if fire_ticks < 200 { fire_ticks as f32 / 200.0 } else { 1.0 };
    if fire_resistance { bonus /= 2.0; }
    bonus = bonus * level as f32 * 0.2;
    let factor = 1.0 + bonus;
    factor.is_finite().then_some(factor)
}

pub(super) fn damage(base: f32, flat: f32, factor: f32) -> Option<f32> {
    if !base.is_finite() || !flat.is_finite() || !factor.is_finite() || factor < 1.0 { return None; }
    let result = (base + flat) * factor;
    result.is_finite().then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remaining_fire_scales_to_ten_seconds_and_resistance_halves_it() {
        assert_eq!(factor(0,false,2),Some(1.0));
        assert_eq!(factor(-20,false,2),Some(1.0));
        assert_eq!(factor(100,false,1),Some(1.1));
        assert_eq!(factor(200,false,1),Some(1.2));
        assert_eq!(factor(300,false,1),Some(1.2));
        assert_eq!(factor(200,true,1),Some(1.1));
        assert_eq!(factor(200,false,2),Some(1.4));
        assert_eq!(factor(200,true,2),Some(1.2));
        assert_eq!(factor(200,false,0),None);
    }
    #[test]
    fn flat_modifiers_participate_and_nonfinite_values_are_rejected() {
        assert_eq!(damage(5.0,2.0,1.2),Some(7.0 * 1.2));
        assert_eq!(damage(5.0,-1.0,1.2),Some(4.0 * 1.2));
        assert_eq!(damage(5.0,0.0,1.0),Some(5.0));
        assert_eq!(damage(5.0,f32::NAN,1.2),None);
        assert_eq!(damage(f32::MAX,f32::MAX,1.2),None);
    }
}
