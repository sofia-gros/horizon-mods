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
