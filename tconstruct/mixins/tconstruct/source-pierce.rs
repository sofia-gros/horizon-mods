use super::tconstruct_world::World;

pub(super) fn supported_definition(world: &World) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes) = raw.0.get("tconstruct:tinkering/modifiers/pierce").and_then(|layers| layers.last()) else { return false; };
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value|
        value == serde_json::json!({"level_display":"tconstruct:default","modules":[
            {"type":"tconstruct:stat_boost","each_level":0.5,"operation":"add","stat":"tconstruct:attack_damage"},
            {"type":"tconstruct:weapon_mob_effect","apply_before_melee":true,"effect":"tconstruct:pierce",
             "level":{"per_level":1.0},"time":{"flat":80.0}}
        ]}))
}

/// MobEffectModule uses Java Math.round, then converts one-based level to amplifier.
pub(super) fn amplifier(level: f32) -> Option<i32> {
    if !level.is_finite() || level <= 0.0 { return None; }
    let rounded = (level + 0.5).floor();
    (rounded >= 1.0 && rounded <= 256.0).then_some(rounded as i32 - 1)
}

pub(super) fn armor_penalty(ticks: i32, amplifier: i32) -> Option<f32> {
    if (ticks == -1 || ticks > 0) && (0..=255).contains(&amplifier) { Some((amplifier + 1) as f32) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_levels_scale_damage_and_round_effect_level() {
        assert_eq!(super::super::tconstruct_source_upgrade_rules::pierce_effective_level(2,1),Some(3.0 + -9.0 / 10.0));
        assert_eq!(super::super::tconstruct_source_upgrade_rules::pierce_effective_level(0,30),Some(3.0));
        assert_eq!(super::super::tconstruct_source_upgrade_rules::pierce_effective_level(0,31),None);
        assert_eq!(super::super::tconstruct_source_upgrade_rules::pierce_effective_level(i32::MAX,1),None);
        assert_eq!(amplifier(super::super::tconstruct_source_upgrade_rules::pierce_effective_level(0,4).unwrap()),None);
        assert_eq!(amplifier(super::super::tconstruct_source_upgrade_rules::pierce_effective_level(0,5).unwrap()),Some(0));
        assert_eq!(amplifier(super::super::tconstruct_source_upgrade_rules::pierce_effective_level(0,15).unwrap()),Some(1));
        assert_eq!(amplifier(super::super::tconstruct_source_upgrade_rules::pierce_effective_level(2,30).unwrap()),Some(4));
        assert_eq!(amplifier(f32::NAN),None);
        assert_eq!(amplifier(257.0),None);
        assert_eq!(armor_penalty(80,2),Some(3.0));
        assert_eq!(armor_penalty(-1,0),Some(1.0));
        assert_eq!(armor_penalty(0,2),None);
        assert_eq!(armor_penalty(80,-1),None);
    }
}
