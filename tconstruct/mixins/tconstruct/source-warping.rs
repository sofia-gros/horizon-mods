// Source SlingTeleportModule's geometry; no native collision/raycast substitute.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Stats {
    pub draw_ticks: i32,
    pub power: f32,
    pub inaccuracy: f32,
}
pub(super) fn stock(item: &str) -> bool {
    matches!(
        item,
        "tconstruct:pickaxe"
            | "tconstruct:sledge_hammer"
            | "tconstruct:vein_hammer"
            | "tconstruct:mattock"
            | "tconstruct:pickadze"
            | "tconstruct:excavator"
            | "tconstruct:hand_axe"
            | "tconstruct:broad_axe"
            | "tconstruct:kama"
            | "tconstruct:scythe"
            | "tconstruct:dagger"
            | "tconstruct:sword"
            | "tconstruct:cleaver"
    )
}
pub(super) fn stats(bytes: &[u8]) -> Option<Stats> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let keys = [
        "tconstruct:draw_speed",
        "tconstruct:projectile_damage",
        "tconstruct:velocity",
        "tconstruct:accuracy",
    ];
    let mut base = [1.0f32, 2.0, 1.0, 0.75];
    let mut multipliers = [1.0f32; 4];
    for module in value.get("modules")?.as_array()? {
        let kind = module.get("type")?.as_str()?;
        let fields = match kind {
            "tconstruct:base_stats" => module.get("stats")?.as_object()?,
            "tconstruct:multiply_stats" => module.get("multipliers")?.as_object()?,
            "tconstruct:part_stats"
            | "tconstruct:material_stats"
            | "tconstruct:default_materials"
            | "tconstruct:modifier_slots"
            | "tconstruct:traits"
            | "tconstruct:is_effective"
            | "tconstruct:box_aoe"
            | "tconstruct:vein_aoe"
            | "tconstruct:circle_aoe"
            | "tconstruct:circle_melee"
            | "tconstruct:sweep_melee"
            | "tconstruct:tool_actions"
            | "tconstruct:material_name"
            | "tconstruct:unique_material_name"
            | "tconstruct:fixed_material_name"
            | "tconstruct:melee_particle"
            | "tconstruct:material_traits"
            | "tconstruct:max_tier"
            | "tconstruct:attack_interaction"
            | "tconstruct:mining_speed_modifier"
            | "tconstruct:conditional_aoe"
            | "tconstruct:volatile_flag"
            | "tconstruct:volatile_int" => continue,
            _ => return None,
        };
        for (i, key) in keys.into_iter().enumerate() {
            if let Some(v) = fields.get(key) {
                let v = v.as_f64()? as f32;
                if !v.is_finite() {
                    return None;
                }
                if kind == "tconstruct:base_stats" {
                    base[i] = v;
                } else {
                    multipliers[i] *= v;
                }
            }
        }
    }
    for i in 0..4 {
        base[i] *= multipliers[i];
        if !base[i].is_finite() {
            return None;
        }
    }
    let draw = base[0].clamp(0.0, 1024.0);
    let power = base[1].clamp(0.0, 1024.0) / 2.0 * base[2].clamp(0.0, 1024.0);
    // Bound unusual packs rather than scan millions of blocks synchronously.
    if power > 32.0 {
        return None;
    }
    Some(Stats {
        draw_ticks: (30.0 / draw).ceil() as i32,
        power,
        inaccuracy: 3.0 * (1.0 / base[3].clamp(0.1, 1.0) - 1.0) * 0.0075,
    })
}
pub(super) fn charge(ticks: i32, draw: i32) -> f32 {
    if ticks <= 0 {
        return 0.0;
    }
    let charge = ticks as f32 / draw as f32;
    ((charge * charge + charge * 2.0) / 3.0).min(1.0)
}
/// The callback returns (within border, suffocating, unbreakable), or None for
/// an unavailable column. A missing column aborts instead of being read as air.
pub(super) fn target(
    position: [f64; 3],
    mut offset: [f64; 3],
    mut block: impl FnMut([i32; 3]) -> Option<(bool, bool, bool)>,
) -> Option<[f64; 3]> {
    if !position.into_iter().chain(offset).all(f64::is_finite)
        || offset.into_iter().any(|v| v.abs() > 1024.0)
    {
        return None;
    }
    let mut furthest = None;
    while offset.iter().any(|v| v.abs() > 0.5) {
        let at = std::array::from_fn(|i| (position[i] + offset[i]).floor() as i32);
        let (border, suffocating, unbreakable) = block(at)?;
        if furthest.is_none() {
            if border && !suffocating {
                furthest = Some(at);
            }
        } else if unbreakable {
            furthest = None;
        }
        for value in &mut offset {
            if value.abs() > 0.25 {
                *value -= value.signum() * 0.25;
            }
        }
    }
    furthest.map(|[x, y, z]| [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5])
}
/// This first held path leaves competing interactions and sling/conditional
/// hooks closed instead of silently ignoring them.
pub(super) fn competing(value: &serde_json::Value) -> bool {
    let Some(modules) = value.get("modules").and_then(|v| v.as_array()) else {
        return true;
    };
    modules.iter().any(|module| {
        let kind = module.get("type").and_then(|v| v.as_str()).unwrap_or("");
        // Module registrations supply implicit hooks; absence of a `hooks`
        // field is not evidence that a module has no item-use interaction.
        !matches!(
            kind,
            "tconstruct:repair"
                | "tconstruct:reduce_tool_damage"
                | "tconstruct:tool_damage_range"
                | "tconstruct:rarity"
                | "tconstruct:stat_boost"
                | "tconstruct:set_stat"
                | "tconstruct:modifier_slot"
                | "tconstruct:volatile_flag"
                | "tconstruct:volatile_int"
                | "tconstruct:volatile_float"
                | "tconstruct:attribute"
                | "tconstruct:conditional_melee_damage"
                | "tconstruct:conditional_mining_speed"
                | "tconstruct:weapon_mob_effect"
                | "tconstruct:lifesteal"
                | "tconstruct:constant_enchantment"
                | "tconstruct:armor_harvest_enchantment"
                | "tconstruct:weapon_looting"
                | "tconstruct:armor_looting"
                | "tconstruct:overgrowth"
                | "tconstruct:armor_level"
                | "tconstruct:show_interaction_source"
        ) || module.get("hooks").is_some_and(|v| {
            let hooks = v.to_string();
            hooks.contains("sling")
                || hooks.contains("general_interact")
                || hooks.contains("conditional_stat")
        })
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn implicit_and_unknown_interactions_fail_closed() {
        for kind in [
            "tconstruct:fishing",
            "tconstruct:shears",
            "tconstruct:place_fire",
            "custom:module",
            "tconstruct:conditional_stat",
        ] {
            assert!(competing(&serde_json::json!({"modules": [{"type": kind}]})));
        }
        assert!(!competing(
            &serde_json::json!({"modules": [{"type": "tconstruct:repair"}]})
        ));
        assert!(competing(
            &serde_json::json!({"modules": [{"type": "tconstruct:repair", "hooks": ["tconstruct:sling_launch"]}]})
        ));
    }
    #[test]
    fn charge_curve_keeps_partial_release_and_zero_speed() {
        assert_eq!(charge(0, 30), 0.0);
        assert!(charge(1, 30) > 0.0);
        assert_eq!(charge(30, 30), 1.0);
        assert_eq!(charge(500, 30), 1.0);
        assert_eq!(charge(1, 0), 1.0);
        let s = stats(br#"{"modules":[{"type":"tconstruct:base_stats","stats":{}}]}"#).unwrap();
        assert_eq!((s.draw_ticks, s.power), (30, 1.0));
    }
    #[test]
    fn chooses_furthest_free_cell_and_unbreakable_resets_it() {
        assert_eq!(
            target([0.0; 3], [0.0, 1.0, 6.0], |_| Some((true, false, false))),
            Some([0.5, 1.0, 6.5])
        );
        let result = target([0.0; 3], [0.0, 1.0, 6.0], |[_, _, z]| {
            Some((true, z == 3, z == 3))
        });
        assert!(result.unwrap()[2] < 3.0);
        assert!(target([0.0; 3], [0.0, 1.0, 6.0], |_| None).is_none());
    }
    #[test]
    fn ignores_suffocation_after_first_candidate_and_checks_border_on_selection() {
        assert_eq!(
            target([0.0; 3], [0.0, 1.0, 6.0], |[_, _, z]| Some((
                true,
                z == 3,
                false
            ))),
            Some([0.5, 1.0, 6.5])
        );
        assert!(target([0.0; 3], [0.0, 1.0, 6.0], |_| Some((false, false, false))).is_none());
    }
}
