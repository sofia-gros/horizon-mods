// Store source mining stats and ordered dynamic traits during regeneration.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use std::collections::BTreeMap;

pub(super) fn apply(stack: &mut ItemStack, stats: &super::tconstruct_source_station_stats::BaseStats,
    levels: &BTreeMap<String, i32>, order: &[String], harvest: bool) -> Option<()> {
    if !harvest { return Some(()); }
    let level = |id: &str| levels.get(id).copied().unwrap_or(0);
    let vintage = level("tconstruct:vintage");
    let speed = super::tconstruct_source_mining_math::base_speed(stats.mining_speed, stats.mining_speed_add_multiplier,
        level("tconstruct:lightweight"), level("tconstruct:heavy"), vintage, level("tconstruct:entangled") > 0);
    // Some harvest-tagged equipment has no supported per-stack tool rules yet.
    // Preserve assembly instead of rejecting the item when those rules are absent.
    let Some(mut tool) = stack.data().and_then(|data| data.other("minecraft:tool")).cloned() else { return Some(()); };
    let Tag::Compound(ref mut fields) = tool else { return None; };
    let Some(Tag::List(rules)) = fields.get_mut(&"rules".into()) else { return None; };
    for rule in rules {
        let Tag::Compound(fields) = rule else { continue; };
        if let Some(Tag::Float(value)) = fields.get_mut(&"speed".into()) { *value = speed.to_bits(); }
    }
    let mut custom = stack.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(ref mut fields) = custom else { return None; };
    let dynamic: Vec<Tag> = order.iter().filter_map(|id| {
        if !matches!(id.as_str(), "tconstruct:jagged" | "tconstruct:stonebound" | "tconstruct:maintained") { return None; }
        Some(Tag::compound([("id", Tag::String(id.as_str().into())), ("level", Tag::Int(level(id)))]))
    }).collect();
    fields.insert("tic_mining_traits".into(), Tag::List(dynamic));
    fields.insert("tic_mining_all".into(), Tag::Float((stats.mining_speed_all_multiplier * (1.0 - 0.1 * vintage.max(0) as f32)).to_bits()));
    fields.insert("tic_durability_all".into(), Tag::Float(stats.durability_multiplier_unit.to_bits()));
    stack.data_mut().set_other("minecraft:tool", Some(tool));
    stack.data_mut().set_other("minecraft:custom_data", Some(custom));
    Some(())
}
