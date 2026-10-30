// Incremental recipes used by the source Sharpness and Swiftstrike upgrades.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug)]
pub(super) struct UpgradePlan {
    pub id: String,
    pub amount: i32,
    pub consumed: Vec<(usize, u32)>,
    pub leftover: Option<ItemStack>,
}

fn amount(tool: &ItemStack, id: &str) -> i32 {
    match tool
        .data()
        .and_then(|d| d.other("minecraft:custom_data"))
        .and_then(|n| n.get("tic_upgrades"))
        .and_then(|n| n.get(id))
    {
        Some(Tag::Int(value)) if *value > 0 => *value,
        _ => 0,
    }
}

pub(super) fn effective(tool: &ItemStack, id: &str, needed: i32) -> f32 {
    amount(tool, id) as f32 / needed as f32
}

pub(super) fn plan(
    recipes: &BTreeMap<String, Vec<u8>>,
    tags: &HashMap<String, HashSet<String>>,
    tool: &ItemStack,
    inputs: &[Option<ItemStack>],
    upgrade_slots: i32,
) -> Option<UpgradePlan> {
    let occupied = [("tconstruct:sharpness", 36), ("tconstruct:swiftstrike", 72)]
        .iter()
        .map(|(id, needed)| (amount(tool, id) + needed - 1) / needed)
        .sum::<i32>();
    for bytes in recipes.values() {
        let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("tconstruct:incremental_modifier") {
            continue;
        }
        let Some(id) = value.get("result").and_then(Value::as_str) else {
            continue;
        };
        if !matches!(id, "tconstruct:sharpness" | "tconstruct:swiftstrike") {
            continue;
        }
        let Some(tool_tag) = value.pointer("/tools/tag").and_then(Value::as_str) else {
            continue;
        };
        if !tags
            .get(tool_tag)
            .is_some_and(|items| items.contains(tool.item.as_ref()))
        {
            continue;
        }
        let Some(input) = value.get("input") else {
            continue;
        };
        let matches = |stack: &ItemStack| -> bool {
            input
                .get("item")
                .and_then(Value::as_str)
                .is_some_and(|id| id == &*stack.item)
                || input.get("tag").and_then(Value::as_str).is_some_and(|tag| {
                    tags.get(tag)
                        .is_some_and(|items| items.contains(stack.item.as_ref()))
                })
        };
        let present: Vec<_> = inputs
            .iter()
            .enumerate()
            .filter_map(|(i, stack)| stack.as_ref().map(|stack| (i + 1, stack)))
            .collect();
        if present.is_empty() || present.iter().any(|(_, stack)| !matches(stack)) {
            continue;
        }
        let Some(needed) = value
            .get("needed_per_level")
            .and_then(Value::as_i64)
            .and_then(|v| i32::try_from(v).ok())
            .filter(|v| *v > 0)
        else {
            continue;
        };
        let Some(per_item) = value
            .get("amount_per_item")
            .and_then(Value::as_i64)
            .and_then(|v| i32::try_from(v).ok())
            .filter(|v| *v > 0)
        else {
            continue;
        };
        let Some(max) = value
            .pointer("/level/max")
            .and_then(Value::as_i64)
            .and_then(|v| i32::try_from(v).ok())
        else {
            continue;
        };
        let cost = value
            .pointer("/slots/upgrades")
            .and_then(Value::as_i64)
            .and_then(|v| i32::try_from(v).ok())
            .unwrap_or(0);
        let old = amount(tool, id);
        if old >= needed * max || (old % needed == 0 && occupied + cost > upgrade_slots) {
            continue;
        }
        let remaining = needed - old % needed;
        let available = present
            .iter()
            .map(|(_, stack)| stack.count as i32 * per_item)
            .sum::<i32>();
        let used = available.min(remaining);
        if used <= 0 {
            continue;
        }
        let mut count = (used + per_item - 1) / per_item;
        let mut consumed = vec![(0, 1)];
        for (index, stack) in present {
            let take = (stack.count as i32).min(count);
            if take > 0 {
                consumed.push((index, take as u32));
                count -= take;
            }
        }
        let excess = (per_item - used % per_item) % per_item;
        let leftover = if excess > 0 {
            let name = value.get("leftover").and_then(Value::as_str)?;
            Some(ItemStack::new(name, excess as u32))
        } else {
            None
        };
        return Some(UpgradePlan {
            id: id.to_owned(),
            amount: old + used,
            consumed,
            leftover,
        });
    }
    None
}

/// Rebuild additive and multiplier effects from the source baseline and saved progress.
pub(super) fn apply_stats(
    tool: &mut ItemStack,
    stats: &super::tconstruct_source_station_stats::BaseStats,
    ductile: i32,
) -> Option<()> {
    let sharpness = effective(tool, "tconstruct:sharpness", 36);
    let swiftstrike = effective(tool, "tconstruct:swiftstrike", 72);
    if sharpness == 0.0 && swiftstrike == 0.0 {
        return Some(());
    }
    let mut custom = tool.data()?.other("minecraft:custom_data")?.clone();
    let attack = match custom.get("tic_attack_damage") {
        Some(Tag::Float(bits)) => Some(f32::from_bits(*bits)),
        _ => None,
    };
    let Tag::Compound(ref mut fields) = custom else {
        return None;
    };
    if sharpness > 0.0 {
        let attack = attack?;
        let multiplier = stats.attack_damage_multiplier
            + stats.attack_damage_multiplier_unit * 0.05 * ductile as f32;
        let damage = attack + 0.75 * sharpness * multiplier;
        fields.insert("tic_attack_damage".into(), Tag::Float(damage.to_bits()));
    }
    if swiftstrike > 0.0 {
        let speed = stats.attack_speed + stats.attack_speed_boost_unit * 0.05 * swiftstrike;
        fields.insert("tic_attack_speed".into(), Tag::Float(speed.to_bits()));
    }
    tool.data_mut()
        .set_other("minecraft:custom_data", Some(custom));
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(amount: i32) -> ItemStack {
        let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
        tool.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([(
                "tic_upgrades",
                Tag::compound([("tconstruct:sharpness", Tag::Int(amount))]),
            )])),
        );
        tool
    }

    #[test]
    fn partial_levels_apply_source_damage_and_attack_speed() {
        let definition = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle"]},{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_speed":1.2}}]}"#;
        let head = serde_json::json!({"tconstruct:head":{"durability":100,"melee_attack":4}})
            .as_object()
            .unwrap()
            .clone();
        let handle = serde_json::json!({"tconstruct:handle":{"melee_damage":0.25}})
            .as_object()
            .unwrap()
            .clone();
        let stats =
            super::super::tconstruct_source_station_stats::base_stats(definition, &[head, handle])
                .unwrap();
        let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
        tool.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([
                ("tic_attack_damage", Tag::Float(5.0f32.to_bits())),
                ("tic_attack_speed", Tag::Float(1.2f32.to_bits())),
                (
                    "tic_upgrades",
                    Tag::compound([
                        ("tconstruct:sharpness", Tag::Int(18)),
                        ("tconstruct:swiftstrike", Tag::Int(36)),
                    ]),
                ),
            ])),
        );
        apply_stats(&mut tool, &stats, 0).unwrap();
        let data = tool.data().unwrap().other("minecraft:custom_data").unwrap();
        let Some(Tag::Float(damage)) = data.get("tic_attack_damage") else {
            panic!("missing damage")
        };
        let Some(Tag::Float(speed)) = data.get("tic_attack_speed") else {
            panic!("missing speed")
        };
        assert!((f32::from_bits(*damage) - 5.46875).abs() < 0.00001);
        assert!((f32::from_bits(*speed) - 1.23).abs() < 0.00001);
    }

    #[test]
    fn swiftstrike_uses_both_input_slots_and_source_tool_tag() {
        let recipe = br#"{"type":"tconstruct:incremental_modifier","amount_per_item":1,"needed_per_level":72,"input":{"item":"minecraft:amethyst_shard"},"result":"tconstruct:swiftstrike","level":{"max":5},"slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/melee/weapon"}}"#.to_vec();
        let recipes = BTreeMap::from([("swift".into(), recipe)]);
        let tags = HashMap::from([(
            "tconstruct:modifiable/melee/weapon".into(),
            HashSet::from(["tconstruct:pickaxe".into()]),
        )]);
        let inputs = [
            Some(ItemStack::new("minecraft:amethyst_shard", 64)),
            Some(ItemStack::new("minecraft:amethyst_shard", 8)),
        ];
        let output = plan(&recipes, &tags, &tool(0), &inputs, 3).unwrap();
        assert_eq!(output.amount, 72);
        assert_eq!(output.consumed, [(0, 1), (1, 64), (2, 8)]);
        assert!(plan(&recipes, &HashMap::new(), &tool(0), &inputs, 3).is_none());
        let third = [None, None, Some(ItemStack::new("minecraft:amethyst_shard", 8))];
        let output = plan(&recipes, &tags, &tool(0), &third, 3).unwrap();
        assert_eq!(output.amount, 8);
        assert_eq!(output.consumed, [(0, 1), (3, 8)]);
    }

    #[test]
    fn incremental_slots_and_block_remainder_follow_source_recipe() {
        let shard = br#"{"type":"tconstruct:incremental_modifier","amount_per_item":1,"needed_per_level":36,"input":{"tag":"forge:gems/quartz"},"result":"tconstruct:sharpness","level":{"max":5},"slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/melee"}}"#.to_vec();
        let block = br#"{"type":"tconstruct:incremental_modifier","amount_per_item":4,"needed_per_level":36,"input":{"item":"minecraft:quartz_block"},"leftover":"minecraft:quartz","result":"tconstruct:sharpness","level":{"max":5},"slots":{"upgrades":1},"tools":{"tag":"tconstruct:modifiable/melee"}}"#.to_vec();
        let tags = HashMap::from([
            (
                "forge:gems/quartz".into(),
                HashSet::from(["minecraft:quartz".into()]),
            ),
            (
                "tconstruct:modifiable/melee".into(),
                HashSet::from(["tconstruct:pickaxe".into()]),
            ),
        ]);
        let recipes = BTreeMap::from([("shard".into(), shard), ("block".into(), block)]);
        let inputs = [Some(ItemStack::new("minecraft:quartz", 35)), None];
        let first = plan(&recipes, &tags, &tool(0), &inputs, 1).unwrap();
        assert_eq!(first.amount, 35);
        assert_eq!(first.consumed, [(0, 1), (1, 35)]);
        let last = plan(
            &recipes,
            &tags,
            &tool(35),
            &[Some(ItemStack::new("minecraft:quartz_block", 1)), None],
            1,
        )
        .unwrap();
        assert_eq!(last.amount, 36);
        assert_eq!(last.leftover.unwrap().count, 3);
        assert!(
            plan(
                &recipes,
                &tags,
                &tool(36),
                &[Some(ItemStack::new("minecraft:quartz", 1)), None],
                1
            )
            .is_none()
        );
        assert!(
            plan(
                &recipes,
                &tags,
                &tool(0),
                &[Some(ItemStack::new("minecraft:dirt", 1)), None],
                1
            )
            .is_none()
        );
        assert!(plan(&recipes, &tags, &tool(0), &inputs, 0).is_none());
        assert!(
            plan(
                &recipes,
                &tags,
                &tool(180),
                &[Some(ItemStack::new("minecraft:quartz", 1)), None],
                6
            )
            .is_none()
        );
    }
}
