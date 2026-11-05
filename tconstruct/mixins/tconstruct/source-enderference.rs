use super::tconstruct_world::World;

pub(super) fn supported_definition(world: &World) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes) = raw.0.get("tconstruct:tinkering/modifiers/enderference").and_then(|layers| layers.last()) else { return false; };
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value| supported_value(&value))
}

fn supported_value(value: &serde_json::Value) -> bool {
    value == &serde_json::json!({"level_display":"tconstruct:default","modules":[
        {"type":"tconstruct:volatile_flag","flag":"tconstruct:enderference",
         "hooks":["tconstruct:projectile_launch","tconstruct:projectile_shot","tconstruct:projectile_thrown"]},
        {"type":"tconstruct:weapon_mob_effect","apply_before_melee":true,
         "effect":"tconstruct:enderference","level":{"flat":1.0},"time":{"flat":100.0}},
        {"type":"tconstruct:counter_mob_effect","chance":{"each_level":0.25},
         "effect":"tconstruct:enderference","level":{"flat":1.0},"time":{"flat":100.0},
         "tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}}
    ]})
}

pub(super) fn active(ticks: i32) -> bool { ticks == -1 || ticks > 0 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effects_expire_but_infinite_effects_remain_active() {
        assert!(active(100));
        assert!(active(1));
        assert!(active(-1));
        assert!(!active(0));
        assert!(!active(-2));
    }
    #[test]
    fn modified_or_missing_modules_do_not_silently_use_stock_semantics() {
        let mut value=serde_json::json!({"level_display":"tconstruct:default","modules":[
            {"type":"tconstruct:volatile_flag","flag":"tconstruct:enderference",
             "hooks":["tconstruct:projectile_launch","tconstruct:projectile_shot","tconstruct:projectile_thrown"]},
            {"type":"tconstruct:weapon_mob_effect","apply_before_melee":true,
             "effect":"tconstruct:enderference","level":{"flat":1.0},"time":{"flat":100.0}},
            {"type":"tconstruct:counter_mob_effect","chance":{"each_level":0.25},
             "effect":"tconstruct:enderference","level":{"flat":1.0},"time":{"flat":100.0},
             "tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}}
        ]});
        assert!(supported_value(&value));
        value["modules"][1]["time"]["flat"]=serde_json::json!(200.0);
        assert!(!supported_value(&value));
        assert!(!supported_value(&serde_json::json!({"modules":[]})));
    }
}
