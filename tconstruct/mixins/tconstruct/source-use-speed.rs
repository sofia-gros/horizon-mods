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
struct Levels {
    ordinary: [i32; 3],
    melee_protection: f32,
}
#[derive(Clone, Copy, Default)]
struct Equipped {
    levels: Levels,
    worn: bool,
    armor: bool,
    active: bool,
}
fn speed(tool: Option<f32>, equipped: &[Equipped]) -> Option<f32> {
    let mut result = 0.2f64 + f64::from(tool.unwrap_or(0.2) - 0.2f32);
    let mut protection = 0.0f32;
    for piece in equipped {
        if piece.levels.ordinary.iter().any(|n| *n < 0)
            || !piece.levels.melee_protection.is_finite() || piece.levels.melee_protection < 0.0 {
            return None;
        }
        if !piece.active {
            continue;
        }
        if piece.worn {
            result += f64::from(0.1f32 * piece.levels.ordinary[0] as f32);
        }
        result += f64::from(0.1f32 * piece.levels.ordinary[1] as f32);
        if piece.armor {
            result += f64::from(0.05f32 * piece.levels.ordinary[2] as f32);
        }
        protection = protection.max(piece.levels.melee_protection);
    }
    result += f64::from(0.05f32 * protection);
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
/// Acquired defense progress uses five units per complete level. Match the
/// source started-level + fractional correction, without rounding it upward.
fn protection_level(stack: &ItemStack, trait_level: i32) -> Option<f32> {
    use horizon_world::nbt::Tag;
    if trait_level < 0 { return None; }
    let raw=match stack.data().and_then(|data|data.other("minecraft:custom_data"))
        .and_then(|custom|custom.get("tic_upgrades")) {
        None => 0,
        Some(upgrades @ Tag::Compound(_)) => match upgrades.get("tconstruct:melee_protection") {
            None => 0, Some(Tag::Int(raw)) if (0..=5 * 32767).contains(raw) => *raw, _ => return None,
        },
        _ => return None,
    };
    let whole=trait_level.checked_add(raw / 5)?;
    let remainder=raw % 5;
    Some(if remainder == 0 { whole as f32 }
        else { whole.checked_add(1)? as f32 + (remainder - 5) as f32 / 5.0 })
}

fn levels(game: &super::Game<'_>, stack: &ItemStack) -> Option<Levels> {
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    let mut result = Levels::default();
    for (i, id) in IDS.iter().enumerate() {
        let trait_level=traits.levels.get(*id).copied().unwrap_or(0);
        let active=if i == 3 {
            result.melee_protection = protection_level(stack,trait_level)?;
            result.melee_protection > 0.0
        } else {
            // The other three purchased modifiers remain unsupported. Their
            // integer trait arithmetic is unchanged by defense progress.
            let total=super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,id)?;
            if total != trait_level || trait_level < 0 { return None; }
            result.ordinary[i]=trait_level;
            trait_level > 0
        };
        if active {
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
        Some(tool_stat(definition, levels.ordinary[0])?)
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
            levels.melee_protection = 0.0;
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
                levels: Levels { ordinary: [2, 1, 2], melee_protection: 3.0 },
                worn: true,
                armor: true,
                active: true,
            },
            Equipped {
                levels: Levels { ordinary: [8, 0, 9], melee_protection: 1.0 },
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
    fn acquired_melee_protection_preserves_fractional_levels_and_maximum() {
        use horizon_world::nbt::Tag;
        let make=|raw:Tag| {
            let mut stack=ItemStack::new("tconstruct:plate_helmet",1);
            stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
                ("tic_upgrades",Tag::compound([("tconstruct:melee_protection",raw)])),
            ]))); stack
        };
        assert_eq!(protection_level(&make(Tag::Int(5)),0),Some(1.0));
        assert_eq!(protection_level(&make(Tag::Int(1)),0),Some(1.0 + -4.0 / 5.0));
        assert_eq!(protection_level(&make(Tag::Int(7)),2),Some(4.0 + -3.0 / 5.0));
        for raw in [Tag::Int(-1),Tag::Int(5*32767+1),Tag::String("5".into())] {
            assert_eq!(protection_level(&make(raw),0),None);
        }
        let pieces=[Equipped {levels:Levels {ordinary:[0;3],melee_protection:0.4},active:true,..Default::default()},
            Equipped {levels:Levels {ordinary:[0;3],melee_protection:1.0},active:true,..Default::default()}];
        assert_eq!(speed(None,&pieces),Some((0.2f64 + f64::from(0.05f32)) as f32));
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
