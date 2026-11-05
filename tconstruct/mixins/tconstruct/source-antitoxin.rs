use super::tconstruct_world::World;

pub(super) fn supported_definition(world: &World) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes) = raw.0.get("tconstruct:tinkering/modifiers/antitoxin").and_then(|layers| layers.last()) else { return false; };
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value|
        value == serde_json::json!({"level_display":"tconstruct:default","modules":[
            {"type":"tconstruct:conditional_melee_damage",
             "attacker":{"type":"mantle:has_effect","effect":"minecraft:poison"},
             "formula":["$poison",0.5,"+","$level","*","$multiplier","*","$damage","+"],
             "percent":false,"variables":{"poison":{"type":"tconstruct:entity","effect":"minecraft:poison",
                "entity_type":"tconstruct:effect_level","fallback":0.0,"which":"attacker"}}},
            {"type":"tconstruct:conditional_stat",
             "entity":{"type":"mantle:has_effect","effect":"minecraft:poison"},
             "formula":["$poison",0.5,"+",0.1,"*","$level","*","$multiplier","*","$value","+"],
             "percent":false,"stat":"tconstruct:draw_speed",
             "variables":{"poison":{"type":"tconstruct:entity","effect":"minecraft:poison",
                "entity_type":"tconstruct:effect_level","fallback":0.0}}}
        ]}))
}

/// EntityEffectLevelVariable is one-based; keep the source postfix evaluation order.
pub(super) fn bonus(amplifier: i32, level: i32, multiplier: f32, draw: bool) -> Option<f32> {
    if !(0..=255).contains(&amplifier) || level <= 0 || !multiplier.is_finite() || multiplier < 0.0 { return None; }
    let poison = (amplifier + 1) as f32 + 0.5;
    let value = if draw { poison * 0.1 * level as f32 * multiplier }
        else { poison * level as f32 * multiplier };
    value.is_finite().then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn poison_amplifier_is_one_based_and_draw_keeps_source_order() {
        assert_eq!(bonus(0,1,1.0,false),Some(1.5));
        assert_eq!(bonus(1,2,1.25,false),Some(6.25));
        assert_eq!(bonus(0,1,1.0,true),Some(1.5 * 0.1));
        assert_eq!(bonus(1,2,0.9,true),Some(2.5 * 0.1 * 2.0 * 0.9));
        assert_eq!(bonus(-1,1,1.0,false),None);
        assert_eq!(bonus(0,0,1.0,false),None);
        assert_eq!(bonus(0,1,f32::NAN,false),None);
    }
}
