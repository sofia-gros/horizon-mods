use super::tconstruct_world::World;

pub(super) fn supported_definition(world: &World) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes) = raw.0.get("tconstruct:tinkering/modifiers/flamestance").and_then(|layers| layers.last()) else { return false; };
    let variable = |which: Option<&str>| {
        let mut value=serde_json::json!({"type":"tconstruct:entity","entity_type":"tconstruct:biome_temperature","fallback":2.0});
        if let Some(which)=which { value["which"]=serde_json::json!(which); }
        serde_json::json!({"temperature":value})
    };
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value|
        value == serde_json::json!({"level_display":"tconstruct:no_levels","modules":[
            {"type":"tconstruct:conditional_melee_damage",
             "formula":["$temperature",0.75,"-","$multiplier","*","$damage","+"],
             "percent":false,"variables":variable(Some("attacker"))},
            {"type":"tconstruct:conditional_stat",
             "formula":["$temperature",0.75,"-",0.6,"*","$multiplier","*","$value","+"],
             "percent":false,"stat":"tconstruct:projectile_damage","variables":variable(None)},
            {"type":"tconstruct:protection","damage_source":"mantle:can_protect",
             "formula":["$temperature",0.75,"-","$protection","+"],
             "tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"},"variables":variable(Some("target"))}
        ]}))
}

/// Flamestance is level-independent and can penalize damage in colder biomes.
pub(super) fn bonus(temperature: f32, multiplier: f32, projectile: bool) -> Option<f32> {
    if !temperature.is_finite() || !multiplier.is_finite() || multiplier < 0.0 { return None; }
    let value = if projectile { (temperature - 0.75) * 0.6 * multiplier }
        else { (temperature - 0.75) * multiplier };
    value.is_finite().then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn warmer_and_colder_temperatures_keep_signed_source_formulas() {
        assert_eq!(bonus(2.0,1.0,false),Some(1.25));
        assert_eq!(bonus(2.0,1.0,true),Some(0.75));
        assert_eq!(bonus(0.75,1.25,false),Some(0.0));
        assert_eq!(bonus(-0.5,1.0,false),Some(-1.25));
        assert_eq!(bonus(-0.5,1.0,true),Some(-0.75));
        assert_eq!(bonus(0.8,1.25,true),Some((0.8 - 0.75) * 0.6 * 1.25));
        assert_eq!(bonus(f32::NAN,1.0,false),None);
        assert_eq!(bonus(2.0,-1.0,false),None);
    }
}
