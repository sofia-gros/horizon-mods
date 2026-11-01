// Source Overslime capacity, refill and damage-shield arithmetic.
// The armor/durability multiplier is the source global durability multiplier.
pub(super) fn capacity(has_overslime: bool, overforced_amount: i32, overforced_trait: i32, overcast: i32, durability_multiplier: f32) -> u32 {
    let base = if has_overslime { 50.0 } else { 0.0 };
    let effective = overforced_amount.max(0) as f32 / 4.0 + overforced_trait.max(0) as f32;
    let overcast = overcast.max(0) as f32;
    ((base + 75.0 * effective + 25.0 * overcast) * (1.0 + 0.5 * overcast) * durability_multiplier.max(0.0)).clamp(0.0, 32767.0) as u32
}

/// Returns (shield remaining, damage passed to later modifiers).
pub(super) fn absorb(current: u32, loss: u32, cause: Option<&str>) -> (u32, u32) {
    if cause == Some("tconstruct:glowing") { return (current, loss); }
    let absorbed = current.min(loss);
    (current - absorbed, loss - absorbed)
}

/// Source caps the stored shield and consumes only enough matching inputs.
pub(super) fn refill(current: u32, cap: u32, per_item: u32, available: u32, overworked: u32) -> Option<(u32, u32)> {
    if current >= cap || per_item == 0 || available == 0 { return None; }
    let each = per_item.saturating_mul(overworked.saturating_add(1));
    let count = (cap - current).div_ceil(each).min(available);
    Some((current.saturating_add(each.saturating_mul(count)).min(cap), count))
}

/// Pinned source tag tconstruct:overslime_friend.
pub(super) fn friendly(levels: &std::collections::BTreeMap<String, i32>, overforced_amount: i32) -> bool {
    overforced_amount > 0 || ["overgrowth", "overcast", "overburn", "overlord", "overshield", "overwield", "overforced", "overslime_friend", "overworked"]
        .iter().any(|id| levels.get(&format!("tconstruct:{id}")).copied().unwrap_or(0) > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capacity_uses_fractional_upgrade_and_source_global_multiplier() {
        assert_eq!(capacity(true, 0, 0, 0, 1.0), 50);
        assert_eq!(capacity(true, 1, 0, 0, 1.0), 68);
        assert_eq!(capacity(true, 20, 0, 0, 1.0), 425);
        assert_eq!(capacity(true, 4, 0, 0, 1.5), 187);
        assert_eq!(capacity(true, 0, 0, 1, 1.0), 112);
    }
    #[test]
    fn shield_precedes_other_wear_and_respects_actual_bypass() {
        assert_eq!(absorb(3, 5, None), (0, 2));
        assert_eq!(absorb(3, 2, None), (1, 0));
        assert_eq!(absorb(3, 2, Some("tconstruct:glowing")), (3, 2));
    }
    #[test]
    fn rebuild_applies_source_friend_penalty_and_clamps_capacity() {
        use horizon_items::items::ItemStack;
        use horizon_world::nbt::Tag;
        let definition = br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:durability":100,"tconstruct:armor":3}}]}"#;
        let stats = super::super::tconstruct_source_station_stats::base_stats(definition, &[]).unwrap();
        let make = |overforced| {
            let mut tool=ItemStack::new("tconstruct:pickaxe",1);
            tool.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
                ("tic_upgrades",Tag::compound([("tconstruct:overslime",Tag::Int(1)),("tconstruct:overforced",Tag::Int(overforced))])),
                ("tic_overslime",Tag::Int(100)),("tic_attack_damage",Tag::Float(5.0f32.to_bits())),
                ("tic_velocity",Tag::Float(2.0f32.to_bits())),("tic_armor",Tag::Float(3.0f32.to_bits())),
            ]))); tool
        };
        let mut plain=make(0);rebuild(&mut plain,&stats,&Default::default(),true,true,true,false).unwrap();
        let fields=plain.data().unwrap().other("minecraft:custom_data").unwrap();
        assert!(matches!(fields.get("tic_overslime"),Some(Tag::Int(50))));
        assert!(matches!(fields.get("tic_attack_damage"),Some(Tag::Float(x)) if (f32::from_bits(*x)-4.5).abs()<0.001));
        assert!(matches!(fields.get("tic_armor"),Some(Tag::Float(x)) if (f32::from_bits(*x)-2.5).abs()<0.001));
        let mut friendly=make(1);rebuild(&mut friendly,&stats,&Default::default(),true,true,true,false).unwrap();
        let fields=friendly.data().unwrap().other("minecraft:custom_data").unwrap();
        assert!(matches!(fields.get("tic_overslime"),Some(Tag::Int(68))));
        assert!(matches!(fields.get("tic_attack_damage"),Some(Tag::Float(x)) if f32::from_bits(*x)==5.0));
    }

    #[test]
    fn refill_caps_without_consuming_unneeded_items() {
        assert_eq!(refill(45, 50, 20, 3, 0), Some((50, 1)));
        assert_eq!(refill(0, 50, 20, 3, 0), Some((50, 3)));
        assert_eq!(refill(0, 50, 20, 3, 1), Some((50, 2)));
        assert_eq!(refill(50, 50, 20, 1, 0), None);
    }
}

pub(super) fn amount(stack: &horizon_items::items::ItemStack, key: &str) -> i32 {
    use horizon_world::nbt::Tag;
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get(key)) {
        Some(Tag::Int(n)) => (*n).max(0), _ => 0,
    }
}

pub(super) fn upgrade(stack: &horizon_items::items::ItemStack, id: &str) -> i32 {
    use horizon_world::nbt::Tag;
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_upgrades")).and_then(|d| d.get(id)) {
        Some(Tag::Int(n)) => (*n).max(0), _ => 0,
    }
}

pub(super) fn rebuild(stack: &mut horizon_items::items::ItemStack,
    stats: &super::tconstruct_source_station_stats::BaseStats,
    levels: &std::collections::BTreeMap<String,i32>, melee: bool, armor: bool, ranged: bool, harvest: bool) -> Option<()> {
    use horizon_world::nbt::Tag;
    let has = upgrade(stack,"tconstruct:overslime") > 0 || levels.get("tconstruct:overslime").copied().unwrap_or(0) > 0;
    let overforced = upgrade(stack,"tconstruct:overforced");
    let cap = capacity(has, overforced, levels.get("tconstruct:overforced").copied().unwrap_or(0), levels.get("tconstruct:overcast").copied().unwrap_or(0), stats.durability_multiplier_unit);
    let unfriendly = has && !friendly(levels, overforced);
    let mut custom = stack.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(ref mut fields) = custom else { return None; };
    fields.insert("tic_overslime_capacity".into(), Tag::Int(cap as i32));
    fields.insert("tic_overslime_active".into(), Tag::Byte(i8::from(has)));
    fields.insert("tic_overslime_overworked".into(), Tag::Int(levels.get("tconstruct:overworked").copied().unwrap_or(0).max(0)));
    if has {
        let amount = match fields.get(&"tic_overslime".into()) { Some(Tag::Int(n)) => (*n).max(0).min(cap as i32), _ => 0 };
        fields.insert("tic_overslime".into(), Tag::Int(amount));
    } else { fields.remove(&"tic_overslime".into()); }
    if unfriendly {
        for (enabled, key) in [(melee,"tic_attack_damage"),(ranged,"tic_velocity")] {
            if enabled { if let Some(Tag::Float(value)) = fields.get_mut(&key.into()) { *value = (f32::from_bits(*value)*0.9).to_bits(); } }
        }
        if armor { if let Some(Tag::Float(value)) = fields.get_mut(&"tic_armor".into()) { *value = (f32::from_bits(*value)-0.5*stats.armor_add_multiplier).max(0.0).to_bits(); } }
        if harvest { if let Some(Tag::Compound(tool)) = fields.get_mut(&"tic_mining_tool".into()) {
            if let Some(Tag::List(rules)) = tool.get_mut(&"rules".into()) { for rule in rules { if let Tag::Compound(rule) = rule { if let Some(Tag::Float(value)) = rule.get_mut(&"speed".into()) { *value = (f32::from_bits(*value)*0.9).max(0.1).to_bits(); } } } }
        } }
    }
    let mining = fields.get(&"tic_mining_tool".into()).cloned();
    stack.data_mut().set_other("minecraft:custom_data", Some(custom));
    if harvest { if let Some(tool) = mining { stack.data_mut().set_other("minecraft:tool", Some(tool)); } }
    Some(())
}

pub(super) fn spend(stack: &mut horizon_items::items::ItemStack, used: u32) {
    use horizon_world::nbt::Tag;
    if used == 0 { return; }
    let Some(mut custom) = stack.data().and_then(|d| d.other("minecraft:custom_data")).cloned() else { return; };
    if let Tag::Compound(ref mut fields) = custom {
        if let Some(Tag::Int(value)) = fields.get_mut(&"tic_overslime".into()) { *value = value.saturating_sub(used as i32).max(0); }
    }
    stack.data_mut().set_other("minecraft:custom_data",Some(custom));
}
