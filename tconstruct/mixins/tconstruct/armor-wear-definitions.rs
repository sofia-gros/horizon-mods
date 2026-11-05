// Canonical JSON definitions: Copyright (c) 2022 SlimeKnights, MIT.
// Source: TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2.
// See the complete notice in tconstruct/third_party/tconstruct/LICENSE.
const KNOWN: &str = r###"{"tconstruct:projectile_protection":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:max_armor_attribute","allow_broken":false,"attribute":"minecraft:generic.knockback_resistance","each_level":0.05,"held_tag":"tconstruct:modifiable/held","operation":"addition"},{"type":"tconstruct:protection","damage_source":{"type":"mantle:and","predicates":["mantle:can_protect",{"type":"mantle:tag","tag":"tconstruct:protection/projectile"}]},"each_level":2.0}]},"tconstruct:iron_armor":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:volatile_flag","flag":"tconstruct:iron_armor"}],"show_in_tooltips":"never"},"tconstruct:fire_protection":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:enchantment_ignoring_protection","level":1,"name":"minecraft:fire_protection"},{"type":"tconstruct:protection","damage_source":{"type":"mantle:and","predicates":["mantle:can_protect",{"type":"mantle:tag","tag":"tconstruct:protection/fire"}]},"each_level":3.0}]},"tconstruct:blast_protection":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:enchantment_ignoring_protection","level":1,"name":"minecraft:blast_protection"},{"type":"tconstruct:protection","damage_source":{"type":"mantle:and","predicates":["mantle:can_protect",{"type":"mantle:tag","tag":"tconstruct:protection/blast"}]},"each_level":3.0}]},"tconstruct:magic_protection":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:max_armor_attribute","allow_broken":false,"attribute":"tconstruct:generic.bad_effect_duration_multiplier","each_level":-0.05,"held_tag":"tconstruct:modifiable/held","operation":"multiply_base"},{"type":"tconstruct:protection","damage_source":{"type":"mantle:and","predicates":["mantle:can_protect",{"type":"mantle:tag","tag":"tconstruct:protection/magic"}]},"each_level":3.0}]},"tconstruct:melee_protection":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:max_armor_attribute","allow_broken":false,"attribute":"tconstruct:player.use_item_speed","each_level":0.05,"held_tag":"tconstruct:modifiable/held","operation":"addition","tooltip_style":"percent"},{"type":"tconstruct:protection","damage_source":{"type":"mantle:and","predicates":["mantle:can_protect",{"type":"mantle:tag","tag":"tconstruct:protection/melee"},{"type":"mantle:inverted","inverted_type":"mantle:is_indirect"}]},"each_level":2.0}]},"tconstruct:knockback_resistance":{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:stat_boost","each_level":0.1,"operation":"add","stat":"tconstruct:knockback_resistance"},{"type":"tconstruct:material_variant_color","material":"tconstruct:wool"}]},"tconstruct:maintained":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:conditional_mining_speed","formula":["$max_durability",0.5,"*","duplicate","$durability","!-","non-negative","!/","$level","*",6.0,"*","$multiplier","*","$speed","+"],"percent":false,"require_effective":true,"variables":{"durability":"tconstruct:tool_durability","max_durability":{"type":"tconstruct:tool_stat","stat":"tconstruct:durability"}}},{"type":"tconstruct:conditional_stat","formula":["$max_durability",0.5,"*","duplicate","$durability","!-","non-negative","!/","$level","*",0.08,"*","$multiplier","*","$value","+"],"percent":false,"stat":"tconstruct:velocity","variables":{"durability":"tconstruct:tool_durability","max_durability":{"type":"tconstruct:tool_stat","stat":"tconstruct:durability"}}},{"type":"tconstruct:attribute","attribute":"minecraft:generic.armor_toughness","formula":["$max_durability",0.5,"*","duplicate","$durability","!-","non-negative","!/","$level","*",2.0,"*"],"operation":"addition","slots":["mainhand","offhand","feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"},"variables":{"durability":"tconstruct:tool_durability","max_durability":{"type":"tconstruct:tool_stat","stat":"tconstruct:durability"}}}]},"tconstruct:fortified":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:modifier_slot","each_level":1,"name":"defense"}],"priority":60},"tconstruct:heavy":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:stat_boost","each_level":0.15,"operation":"multiply_base","stat":"tconstruct:mining_speed"},{"type":"tconstruct:stat_boost","each_level":0.1,"operation":"add","stat":"tconstruct:accuracy"},{"type":"tconstruct:stat_boost","each_level":0.15,"operation":"add","stat":"tconstruct:knockback_resistance"},{"type":"tconstruct:attribute","attribute":"minecraft:generic.knockback_resistance","each_level":0.1,"operation":"multiply_base","slots":["mainhand","offhand","feet","legs","chest","head"],"tool":{"type":"mantle:inverted","inverted_type":"mantle:tag","tag":"tconstruct:modifiable/armor"}},{"type":"tconstruct:attribute","attribute":"minecraft:generic.movement_speed","each_level":-0.1,"operation":"multiply_base","slots":["mainhand","offhand","feet","legs","chest","head"]},{"type":"tconstruct:attribute","attribute":"forge:entity_gravity","each_level":0.05,"operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"}]},"tconstruct:solid":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:stat_boost","each_level":0.15,"operation":"multiply_base","stat":"tconstruct:attack_damage"},{"type":"tconstruct:stat_boost","each_level":0.75,"operation":"add","stat":"tconstruct:projectile_damage"},{"type":"tconstruct:protection","damage_source":"mantle:can_protect","each_level":1.25,"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}},{"type":"tconstruct:attribute","attribute":"minecraft:generic.movement_speed","each_level":-0.1,"operation":"multiply_base","slots":["mainhand","offhand","feet","legs","chest","head"]},{"type":"tconstruct:attribute","attribute":"forge:entity_gravity","each_level":0.05,"operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"}]},"tconstruct:featherweight":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:stat_boost","each_level":0.05,"operation":"add","stat":"tconstruct:draw_speed"},{"type":"tconstruct:stat_boost","each_level":0.05,"operation":"add","stat":"tconstruct:accuracy"},{"type":"tconstruct:protection","damage_source":"mantle:can_protect","each_level":-0.625,"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}},{"type":"tconstruct:attribute","attribute":"minecraft:generic.movement_speed","each_level":0.05,"operation":"multiply_base","slots":["mainhand","offhand","feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}},{"type":"tconstruct:attribute","attribute":"tconstruct:player.use_item_speed","each_level":0.05,"operation":"addition","slots":["mainhand","offhand","feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"},"tooltip_style":"percent"}]},"tconstruct:ductile":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:stat_boost","each_level":0.1,"operation":"multiply_base","stat":"tconstruct:durability"},{"type":"tconstruct:stat_boost","each_level":0.05,"operation":"multiply_base","stat":"tconstruct:attack_damage"},{"type":"tconstruct:stat_boost","each_level":0.25,"operation":"add","stat":"tconstruct:projectile_damage"},{"type":"tconstruct:stat_boost","each_level":1.0,"operation":"add","stat":"tconstruct:armor_toughness"}]},"tconstruct:preserved":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:stat_boost","each_level":0.15,"operation":"multiply_base","stat":"tconstruct:durability"},{"type":"tconstruct:repair","each_level":0.15}]},"tconstruct:netherite":{"level_display":"tconstruct:no_levels","modules":[{"type":"tconstruct:rarity","rarity":"rare"},{"type":"tconstruct:volatile_flag","flag":"tconstruct:indestructible"},{"type":"tconstruct:stat_boost","flat":0.2,"operation":"multiply_base","stat":"tconstruct:durability"},{"type":"tconstruct:stat_boost","flat":1.0,"operation":"add","stat":"tconstruct:armor_toughness"},{"type":"tconstruct:stat_boost","flat":0.05,"operation":"add","stat":"tconstruct:knockback_resistance"},{"type":"tconstruct:stat_boost","flat":0.2,"operation":"multiply_base","stat":"tconstruct:attack_damage"},{"type":"tconstruct:stat_boost","flat":0.25,"operation":"multiply_base","stat":"tconstruct:mining_speed"},{"type":"tconstruct:set_stat","stat":"tconstruct:harvest_tier","value":"minecraft:netherite"},{"type":"tconstruct:stat_boost","flat":0.1,"operation":"multiply_base","stat":"tconstruct:velocity"},{"type":"tconstruct:volatile_flag","flag":"tconstruct:netherite"}]}}"###;

// These source capacity/recovery definitions have no TOOL_DAMAGE or
// DURABILITY_CHANGED hook. Overburn's child is the static TankModule below.
// Equality covers all modules, hook overrides, conditions and priority.
const OVERSLIME_FRIENDS: &str = r###"{
  "tconstruct:overforced": {
    "level_display": "tconstruct:default",
    "modules": [
      {
        "type": "tconstruct:stat_boost",
        "each_level": 75.0,
        "operation": "add",
        "stat": "tconstruct:overslime"
      }
    ]
  },
  "tconstruct:overcast": {
    "level_display": "tconstruct:default",
    "modules": [
      {
        "type": "tconstruct:stat_boost",
        "each_level": 25.0,
        "operation": "add",
        "stat": "tconstruct:overslime"
      },
      {
        "type": "tconstruct:stat_boost",
        "each_level": 0.5,
        "operation": "multiply_base",
        "stat": "tconstruct:overslime"
      }
    ]
  },
  "tconstruct:overgrowth": {
    "level_display": "tconstruct:default",
    "modules": [
      {
        "type": "tconstruct:overgrowth",
        "chance": {
          "each_level": 0.05
        }
      }
    ]
  },
  "tconstruct:overburn": {
    "level_display": "tconstruct:default",
    "modules": [
      {
        "type": "tconstruct:overburn"
      },
      {
        "type": "tconstruct:stat_boost",
        "flat": 1000.0,
        "operation": "add",
        "stat": "tconstruct:tank_capacity"
      },
      {
        "type": "tconstruct:trait",
        "fixed_level": true,
        "level": 1,
        "name": "tconstruct:tank_handler"
      }
    ]
  },
  "tconstruct:overlord": {
    "level_display": "tconstruct:default",
    "modules": [
      {
        "type": "tconstruct:stat_copy",
        "each_level": 0.1,
        "source": "tconstruct:durability",
        "target": "tconstruct:overslime"
      },
      {
        "type": "tconstruct:stat_boost",
        "each_level": -0.15,
        "modifier_level": {
          "max": 6
        },
        "operation": "multiply_base",
        "stat": "tconstruct:durability"
      },
      {
        "type": "tconstruct:stat_boost",
        "flat": -0.99999,
        "modifier_level": {
          "min": 7
        },
        "operation": "multiply_base",
        "stat": "tconstruct:durability"
      }
    ]
  },
  "tconstruct:overslime_friend": {
    "level_display": "tconstruct:no_levels",
    "show_in_tooltips": "advanced"
  }
}"###;

/// Source static registrations cannot be replaced by a same-ID dynamic file.
/// Overworked is Modifier::new (no hooks). TankModule has fluid/build/slot/name
/// hooks only; neither participates in ordinary damage or durability changes.
pub(super) fn static_no_wear(id: &str) -> bool {
    matches!(id, "tconstruct:overworked" | "tconstruct:tank_handler")
}

pub(super) fn supported(id: &str, value: &serde_json::Value) -> bool {
    static DEFINITIONS: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    static FRIENDS: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    DEFINITIONS.get_or_init(|| serde_json::from_str(KNOWN).expect("retained no-wear modifier definitions"))
        .get(id).is_some_and(|expected| expected == value)
        || FRIENDS.get_or_init(|| serde_json::from_str(OVERSLIME_FRIENDS).expect("retained Overslime friend definitions"))
            .get(id).is_some_and(|expected| expected == value)
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn unknown_and_changed_definitions_never_enable_wear() {
        let definitions: serde_json::Value = serde_json::from_str(KNOWN).unwrap();
        let mut iron = definitions["tconstruct:iron_armor"].clone();
        assert!(supported("tconstruct:iron_armor", &iron));
        assert!(!supported("tconstruct:overslime", &iron));
        iron["priority"] = serde_json::json!(200);
        assert!(!supported("tconstruct:iron_armor", &iron));
    }
}

#[cfg(test)] mod overslime_friend_tests {
    use super::*;
    #[test] fn source_capacity_and_recovery_definitions_are_admitted_exactly() {
        let definitions: serde_json::Value = serde_json::from_str(OVERSLIME_FRIENDS).unwrap();
        assert_eq!(definitions.as_object().unwrap().len(), 6);
        for (id, value) in definitions.as_object().unwrap() {
            assert!(supported(id, value), "{id}");
            let mut changed = value.clone();
            changed["priority"] = serde_json::json!(150);
            assert!(!supported(id, &changed), "changed priority: {id}");
            changed = value.clone();
            changed["modules"] = serde_json::json!([
                {"type":"tconstruct:reduce_tool_damage", "percent":{"flat":1.0}}
            ]);
            assert!(!supported(id, &changed), "changed hooks: {id}");
        }
    }
    #[test] fn overburn_requires_the_complete_fixed_child_definition() {
        let definitions: serde_json::Value = serde_json::from_str(OVERSLIME_FRIENDS).unwrap();
        let mut value = definitions["tconstruct:overburn"].clone();
        value["modules"][2]["name"] = serde_json::json!("tconstruct:overshield");
        assert!(!supported("tconstruct:overburn", &value));
        value = definitions["tconstruct:overburn"].clone();
        value["modules"][0]["hooks"] = serde_json::json!(["tconstruct:tool_damage"]);
        assert!(!supported("tconstruct:overburn", &value));
    }
    #[test] fn only_the_two_proven_static_no_wear_registrations_bypass_json() {
        assert!(static_no_wear("tconstruct:overworked"));
        assert!(static_no_wear("tconstruct:tank_handler"));
        for id in ["tconstruct:overslime", "tconstruct:overshield", "tconstruct:overwield",
            "tconstruct:overcast", "tconstruct:unknown", "other:overworked"] {
            assert!(!static_no_wear(id), "{id}");
        }
    }
}
