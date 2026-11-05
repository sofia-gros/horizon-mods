// Source ToolClientEvents.handleInput and its four use-speed modifier modules.
use horizon_items::items::ItemStack;
use serde_json::Value;
const IDS: [&str; 4] = [
    "tconstruct:unburdened",
    "tconstruct:cobalamin",
    "tconstruct:featherweight",
    "tconstruct:melee_protection",
];
const DEFINITIONS: [&str; 4] = [
    r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:stat_boost","each_level":0.1,"operation":"add","stat":"tconstruct:use_item_speed"},{"type":"tconstruct:attribute","attribute":"tconstruct:player.use_item_speed","each_level":0.1,"operation":"addition","slots":["feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor/worn"},"tooltip_style":"percent"}]}"#,
    r#"{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:attribute","attribute":"minecraft:generic.attack_speed","each_level":0.05,"operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"},{"type":"tconstruct:attribute","attribute":"tconstruct:player.use_item_speed","each_level":0.1,"operation":"addition","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"},{"type":"tconstruct:attribute","attribute":"tconstruct:player.mining_speed_multiplier","each_level":0.15,"operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"],"tooltip_style":"percent"}]}"#,
    r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:stat_boost","each_level":0.05,"operation":"add","stat":"tconstruct:draw_speed"},{"type":"tconstruct:stat_boost","each_level":0.05,"operation":"add","stat":"tconstruct:accuracy"},{"type":"tconstruct:protection","damage_source":"mantle:can_protect","each_level":-0.625,"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}},{"type":"tconstruct:attribute","attribute":"minecraft:generic.movement_speed","each_level":0.05,"operation":"multiply_base","slots":["mainhand","offhand","feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}},{"type":"tconstruct:attribute","attribute":"tconstruct:player.use_item_speed","each_level":0.05,"operation":"addition","slots":["mainhand","offhand","feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"},"tooltip_style":"percent"}]}"#,
    r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:max_armor_attribute","allow_broken":false,"attribute":"tconstruct:player.use_item_speed","each_level":0.05,"held_tag":"tconstruct:modifiable/held","operation":"addition","tooltip_style":"percent"},{"type":"tconstruct:protection","damage_source":{"type":"mantle:and","predicates":["mantle:can_protect",{"type":"mantle:tag","tag":"tconstruct:protection/melee"},{"type":"mantle:inverted","inverted_type":"mantle:is_indirect"}]},"each_level":2.0}]}"#,
];

fn known(index: usize, bytes: Option<&[u8]>) -> bool {
    bytes
        .and_then(|b| serde_json::from_slice::<Value>(b).ok())
        .is_some_and(|v| Some(v) == serde_json::from_str(DEFINITIONS[index]).ok())
}
/// Source default0.2, module base updates and total multipliers. Known material
/// stat classes do not contribute to USE_ITEM_SPEED.
fn tool_stat(definition: &[u8], unburdened: i32) -> Option<f32> {
    if unburdened < 0 {
        return None;
    }
    let value: Value = serde_json::from_slice(definition).ok()?;
    let mut base = 0.2f32;
    let mut add = 0.0f32;
    let mut multiplier = 1.0f32;
    for module in value.get("modules")?.as_array()? {
        match module.get("type")?.as_str()? {
            "tconstruct:base_stats" | "tconstruct:multiply_stats" => {
                let multiplying = module["type"] == "tconstruct:multiply_stats";
                let fields = module
                    .get(if multiplying { "multipliers" } else { "stats" })?
                    .as_object()?;
                if let Some(number) = fields.get("tconstruct:use_item_speed") {
                    let number = number.as_f64()? as f32;
                    if !number.is_finite() {
                        return None;
                    }
                    if multiplying {
                        multiplier *= number;
                    } else {
                        base = 0.0;
                        add += number;
                    }
                }
            }
            "tconstruct:attack_interaction"
            | "tconstruct:box_aoe"
            | "tconstruct:circle_aoe"
            | "tconstruct:circle_melee"
            | "tconstruct:conditional_aoe"
            | "tconstruct:custom_material_name"
            | "tconstruct:default_materials"
            | "tconstruct:dual_option_interaction"
            | "tconstruct:fixed_material_name"
            | "tconstruct:is_effective"
            | "tconstruct:material_name"
            | "tconstruct:material_repair"
            | "tconstruct:material_stats"
            | "tconstruct:material_traits"
            | "tconstruct:max_tier"
            | "tconstruct:melee_particle"
            | "tconstruct:melting_fluid_effective"
            | "tconstruct:mining_speed_modifier"
            | "tconstruct:modifier_slots"
            | "tconstruct:part_stats"
            | "tconstruct:remapping_materials"
            | "tconstruct:statless_part_repair"
            | "tconstruct:sweep_melee"
            | "tconstruct:toggleable_set_interaction"
            | "tconstruct:tool_actions"
            | "tconstruct:tool_parts"
            | "tconstruct:traits"
            | "tconstruct:unique_material_name"
            | "tconstruct:vein_aoe"
            | "tconstruct:volatile_flag"
            | "tconstruct:volatile_int" => {}
            _ => return None,
        }
    }
    let result = (base + add + 0.1f32 * unburdened as f32) * multiplier;
    result.is_finite().then(|| result.clamp(0.0, 1.0))
}
#[derive(Clone, Copy, Default)]
struct Equipped {
    levels: [i32; 4],
    worn: bool,
    armor: bool,
    active: bool,
}
fn speed(tool: Option<f32>, equipped: &[Equipped]) -> Option<f32> {
    let mut result = 0.2f64 + f64::from(tool.unwrap_or(0.2) - 0.2f32);
    let mut protection = 0;
    for piece in equipped {
        if piece.levels.iter().any(|n| *n < 0) {
            return None;
        }
        if !piece.active {
            continue;
        }
        if piece.worn {
            result += f64::from(0.1f32 * piece.levels[0] as f32);
        }
        result += f64::from(0.1f32 * piece.levels[1] as f32);
        if piece.armor {
            result += f64::from(0.05f32 * piece.levels[2] as f32);
        }
        protection = protection.max(piece.levels[3]);
    }
    result += f64::from(0.05f32 * protection as f32);
    result.is_finite().then(|| result.clamp(0.0, 1.0) as f32)
}
fn class_slot(item: &str) -> Option<usize> {
    match item {
        "tconstruct:travelers_helmet" | "tconstruct:plate_helmet" | "tconstruct:slime_helmet" => {
            Some(0)
        }
        "tconstruct:travelers_chestplate"
        | "tconstruct:plate_chestplate"
        | "tconstruct:slimy_chestplate"
        | "tconstruct:slime_wings" => Some(1),
        "tconstruct:travelers_leggings"
        | "tconstruct:plate_leggings"
        | "tconstruct:slime_leggings" => Some(2),
        "tconstruct:travelers_boots" | "tconstruct:plate_boots" | "tconstruct:slime_boots" => {
            Some(3)
        }
        _ => None,
    }
}
fn levels(game: &super::Game<'_>, stack: &ItemStack) -> Option<[i32; 4]> {
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    let mut result = [0; 4];
    for (i, id) in IDS.iter().enumerate() {
        // These upgrades have no ordinary Station port yet. Do not guess the
        // unit of a manually stored incremental-defense counter.
        let total =
            super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, id)?;
        let level = traits.levels.get(*id).copied().unwrap_or(0);
        if total != level || level < 0 {
            return None;
        }
        result[i] = level;
        if level > 0 {
            let path = format!(
                "tconstruct:tinkering/modifiers/{}",
                id.strip_prefix("tconstruct:")?
            );
            if !known(
                i,
                raw.0.get(&path).and_then(|v| v.last()).map(Vec::as_slice),
            ) {
                return None;
            }
        }
    }
    // A custom module that also modifies this stat cannot be silently ignored.
    for (id, level) in &traits.levels {
        if *level <= 0 || IDS.contains(&id.as_str()) {
            continue;
        }
        let (namespace, path) = id.split_once(':')?;
        if raw
            .0
            .get(&format!("{namespace}:tinkering/modifiers/{path}"))
            .and_then(|v| v.last())
            .is_some_and(|b| String::from_utf8_lossy(b).contains("use_item_speed"))
        {
            return None;
        }
    }
    Some(result)
}
fn projected(game: &super::Game<'_>, native: Option<f32>) -> Option<f32> {
    if game.remote.is_some() || game.riding.is_some() {
        return None;
    }
    // Stock source replaces vanilla's20% slowdown. Unknown modern use-speed
    // components keep their native behavior instead of being overwritten.
    if native != Some(0.2) {
        return None;
    }
    let (slot, _) = game.using?;
    let using = game.stack_in(slot)?;
    let tags = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    let has = |tag: &str, stack: &ItemStack| {
        tags.get(tag)
            .is_some_and(|v| v.contains(stack.item.as_ref()))
    };
    let tool = if using.item.starts_with("tconstruct:") && has("tconstruct:modifiable/held", using)
    {
        let levels = levels(game, using)?;
        let definitions = game
            .world
            .chunks
            .__horizon_mixins_tconstruct_material_snapshot()
            .as_ref()?;
        let (namespace, name) = using.item.split_once(':')?;
        let name = super::tconstruct_source_station_stats::definition_name(namespace, name);
        let definition = definitions
            .get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?
            .last()?;
        Some(tool_stat(definition, levels[0])?)
    } else {
        None
    };
    let equipment = game
        .armor
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.as_ref()))
        .chain([
            (
                4,
                game.inventory.get(game.selected).and_then(Option::as_ref),
            ),
            (5, game.offhand.as_ref()),
        ]);
    let mut pieces = Vec::new();
    for (index, stack) in equipment {
        let Some(stack) = stack.filter(|s| s.item.starts_with("tconstruct:")) else {
            continue;
        };
        let worn = index < 4 && class_slot(&stack.item) == Some(index);
        let held = index >= 4 && has("tconstruct:modifiable/held", stack);
        if !worn && !held {
            continue;
        }
        if super::tconstruct_source_ranged_wear::is_broken(stack) {
            continue;
        }
        let mut levels = levels(game, stack)?;
        let worn_tag = worn && has("tconstruct:modifiable/armor/worn", stack);
        if worn && !worn_tag {
            levels[3] = 0;
        }
        pieces.push(Equipped {
            levels,
            worn: worn_tag,
            armor: has("tconstruct:modifiable/armor", stack),
            active: true,
        });
    }
    speed(tool, &pieces)
}
pub(super) fn apply(native: Option<f32>, game: &super::Game<'_>) -> Option<f32> {
    projected(game, native).or(native)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn held_stat_uses_defaults_additions_multipliers_and_clamp() {
        let base = br#"{"modules":[]}"#;
        assert_eq!(tool_stat(base, 0), Some(0.2));
        assert_eq!(tool_stat(base, 2), Some(0.4));
        let altered=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:use_item_speed":0.4}},{"type":"tconstruct:base_stats","stats":{"tconstruct:use_item_speed":0.2}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:use_item_speed":0.5}}]}"#;
        assert_eq!(tool_stat(altered, 2), Some((0.4f32 + 0.2 + 0.2) * 0.5));
        assert_eq!(tool_stat(base, 100), Some(1.0));
        assert_eq!(tool_stat(base, -1), None);
        assert_eq!(
            tool_stat(br#"{"modules":[{"type":"custom:unknown"}]}"#, 1),
            None
        );
    }
    #[test]
    fn equipment_adds_attributes_and_only_maximum_protection() {
        let pieces = [
            Equipped {
                levels: [2, 1, 2, 3],
                worn: true,
                armor: true,
                active: true,
            },
            Equipped {
                levels: [8, 0, 9, 1],
                worn: false,
                armor: false,
                active: true,
            },
        ];
        let expected = (0.2f64
            + f64::from(0.4f32 - 0.2)
            + f64::from(0.1f32 * 2.0)
            + f64::from(0.1f32)
            + f64::from(0.05f32 * 2.0)
            + f64::from(0.05f32 * 3.0)) as f32;
        assert_eq!(speed(Some(0.4), &pieces), Some(expected));
        assert_eq!(speed(Some(1.0), &pieces), Some(1.0));
        assert_eq!(
            speed(
                None,
                &[Equipped {
                    active: false,
                    ..pieces[0]
                }]
            ),
            Some(0.2)
        );
    }
    #[test]
    fn exact_source_modules_are_required() {
        for i in 0..4 {
            assert!(known(i, Some(DEFINITIONS[i].as_bytes())));
            assert!(!known(i, None));
            assert!(!known(i, Some(b"{}")));
        }
    }
}
