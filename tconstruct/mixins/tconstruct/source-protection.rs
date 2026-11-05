/// Acquisition contracts only. Protection and its distinct secondary modules
/// are not silently replaced by vanilla protection enchantments.
pub(super) fn supported_definition(world: &super::tconstruct_world::World, id: &str) -> bool {
    let Some(name)=id.strip_prefix("tconstruct:") else { return false; };
    let Some(raw)=world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes)=raw.0.get(&format!("tconstruct:tinkering/modifiers/{name}"))
        .and_then(|layers| layers.last()) else { return false; };
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value| supported_value(name,&value))
}

fn supported_value(name: &str, value: &serde_json::Value) -> bool {
    let (kind, points, secondary) = match name {
        "fire_protection" => ("fire",3.0,serde_json::json!({"type":"tconstruct:enchantment_ignoring_protection","level":1,"name":"minecraft:fire_protection"})),
        "blast_protection" => ("blast",3.0,serde_json::json!({"type":"tconstruct:enchantment_ignoring_protection","level":1,"name":"minecraft:blast_protection"})),
        "magic_protection" => ("magic",3.0,serde_json::json!({"type":"tconstruct:max_armor_attribute","allow_broken":false,
            "attribute":"tconstruct:generic.bad_effect_duration_multiplier","each_level":-0.05,
            "held_tag":"tconstruct:modifiable/held","operation":"multiply_base"})),
        "melee_protection" => ("melee",2.0,serde_json::json!({"type":"tconstruct:max_armor_attribute","allow_broken":false,
            "attribute":"tconstruct:player.use_item_speed","each_level":0.05,
            "held_tag":"tconstruct:modifiable/held","operation":"addition","tooltip_style":"percent"})),
        "projectile_protection" => ("projectile",2.0,serde_json::json!({"type":"tconstruct:max_armor_attribute","allow_broken":false,
            "attribute":"minecraft:generic.knockback_resistance","each_level":0.05,
            "held_tag":"tconstruct:modifiable/held","operation":"addition"})),
        _ => return false,
    };
    let mut predicates=vec![serde_json::json!("mantle:can_protect"),
        serde_json::json!({"type":"mantle:tag","tag":format!("tconstruct:protection/{kind}")})];
    if kind == "melee" { predicates.push(serde_json::json!({"type":"mantle:inverted","inverted_type":"mantle:is_indirect"})); }
    value == &serde_json::json!({"level_display":"tconstruct:default","modules":[secondary,
        {"type":"tconstruct:protection","damage_source":{"type":"mantle:and","predicates":predicates},"each_level":points}]})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_blast_contract_requires_its_secondary_module_and_three_points() {
        let mut value=serde_json::json!({"level_display":"tconstruct:default","modules":[
            {"type":"tconstruct:enchantment_ignoring_protection","level":1,"name":"minecraft:blast_protection"},
            {"type":"tconstruct:protection","damage_source":{"type":"mantle:and","predicates":[
                "mantle:can_protect",{"type":"mantle:tag","tag":"tconstruct:protection/blast"}]},"each_level":3.0}
        ]});
        assert!(supported_value("blast_protection",&value));
        assert!(!supported_value("fire_protection",&value));
        value["modules"][1]["each_level"]=serde_json::json!(2.0);
        assert!(!supported_value("blast_protection",&value));
        assert!(!supported_value("unknown",&value));
    }
}

/// IncrementalModifier uses the started integer level plus its negative
/// incomplete-level correction. Keep that f32 evaluation order.
pub(super) fn effective_level(trait_level: i32, raw: i32) -> Option<f32> {
    if trait_level < 0 || !(0..=5 * 32767).contains(&raw) { return None; }
    let whole=trait_level.checked_add(raw / 5)?;
    let remainder=raw % 5;
    Some(if remainder == 0 { whole as f32 }
        else { whole.checked_add(1)? as f32 + (remainder - 5) as f32 / 5.0 })
}

pub(super) fn projectile_attribute(levels: &[f32]) -> Option<f64> {
    if levels.iter().any(|level| !level.is_finite() || *level < 0.0) { return None; }
    let maximum=levels.iter().copied().fold(0.0f32, f32::max);
    let value=0.05f32 * maximum;
    (value.is_finite() && value > 0.0).then_some(f64::from(value))
}

#[cfg(test)]
mod projectile_attribute_tests {
    use super::*;
    #[test]
    fn partial_progress_uses_source_float_order_and_material_levels_once() {
        assert_eq!(effective_level(0,1),Some(1.0 + -4.0 / 5.0));
        assert_eq!(effective_level(2,7),Some(4.0 + -3.0 / 5.0));
        assert_eq!(effective_level(2,5),Some(3.0));
        assert_eq!(effective_level(0,-1),None);
        assert_eq!(effective_level(0,5 * 32767 + 1),None);
    }
    #[test]
    fn resistance_uses_the_maximum_piece_not_the_sum() {
        assert_eq!(projectile_attribute(&[1.0,2.0,3.0,2.0]),Some(f64::from(0.05f32 * 3.0)));
        assert_eq!(projectile_attribute(&[3.0,3.0]),Some(f64::from(0.05f32 * 3.0)));
        assert_eq!(projectile_attribute(&[effective_level(0,1).unwrap()]),Some(f64::from(0.05f32 * (1.0 + -4.0 / 5.0))));
        assert_eq!(projectile_attribute(&[]),None);
        assert_eq!(projectile_attribute(&[0.0]),None);
        assert_eq!(projectile_attribute(&[f32::NAN]),None);
    }
}
