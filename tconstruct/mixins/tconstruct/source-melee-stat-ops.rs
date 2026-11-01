// Source StatBoostModule operations for the supported melee stat traits.
// Callers provide authoritative trait levels and source tool-tag membership.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MeleeInputs {
    pub damage: f32,
    pub damage_add_multiplier: f32,
    pub speed: f32,
    pub ductile: i32,
    pub solid: i32,
    pub pierce: i32,
    pub lightweight: i32,
    pub vintage: i32,
    pub sharpness: f32,
    pub swiftstrike: f32,
    pub melee: bool,
    pub melee_weapon: bool,
}

pub(super) fn melee_stats(v: MeleeInputs) -> (f32, f32) {
    let damage = if v.melee {
        (v.damage + (0.5 * v.pierce.max(0) as f32 + 0.75 * v.sharpness.max(0.0)) * v.damage_add_multiplier)
            * (1.0 + 0.05 * v.ductile.max(0) as f32)
            * (1.0 + 0.15 * v.solid.max(0) as f32)
    } else { v.damage };
    let speed = if v.melee_weapon {
        v.speed * (1.0 + 0.08 * v.lightweight.max(0) as f32)
            * (1.0 + 0.05 * v.swiftstrike.max(0.0))
            * (1.0 - 0.1 * v.vintage.max(0) as f32)
    } else { v.speed };
    (damage.clamp(0.0, 2048.0), speed.clamp(0.0, 1024.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_operations_combine_from_base_once() {
        let v = MeleeInputs { damage: 5.0, damage_add_multiplier: 1.25, speed: 1.2, ductile: 2, solid: 1, pierce: 1, lightweight: 2, vintage: 1, sharpness: 0.5, swiftstrike: 0.5, melee: true, melee_weapon: true };
        let (damage, speed) = melee_stats(v);
        assert!((damage - 7.708594).abs() < 0.0001);
        assert!((speed - 1.28412).abs() < 0.0001);
        assert_eq!(melee_stats(v), (damage, speed));
    }
    #[test]
    fn unsupported_tool_tags_do_not_receive_stat_effects() {
        let v = MeleeInputs { damage: 3.0, speed: 2.0, pierce: 2, solid: 1, lightweight: 3, vintage: 2, ..Default::default() };
        assert_eq!(melee_stats(v), (3.0, 2.0));
    }
}
