//! Binds ToolStack state (the tic_* data in `custom_data`) to the visible stack: the durability
//! bar ModifiableItem derives from ToolDamageUtil, and the per-stack mining component.
//! SlimeKnights (MIT; see LICENSE) defines the arithmetic; this only mirrors it onto an ItemSlot.
use crate::{
    tool_damage,
    tool_stack::{current_durability, damage, durability, is_broken, is_initialized, mining_component, rebuild_stats, stat, Environment},
    tool_stats::Stat,
};
use pluto_lib::{Identifier, ItemSlot};

/// Mirrors the stored tool state onto the slot: `max_damage` is the fake maximum (one above
/// durability while broken, so vanilla never deletes the stack), `damage` is ToolStack.getDamage,
/// and `tool` is the mining component for the current speed/tier/broken state.
pub(crate) fn sync_visuals(slot: &mut ItemSlot, env: &Environment) -> Result<(), String> {
    crate::tool_materials::sync_model(slot);
    let data = &slot.custom_data;
    slot.max_damage = Some(tool_damage::fake_max_damage(data, true).max(1));
    slot.damage = u32::try_from(damage(data)).unwrap_or(0);
    slot.tool = match env.definition { Some(definition) => mining_component(data, definition)?, None => None };
    slot.attribute_modifiers = attribute_modifiers(data, env);
    // The tool's enchantments come from its modifiers alone; a tool with none keeps whatever component it has (none).
    let levels = crate::tool_stack::enchantments(data, env);
    slot.enchantments = if levels.is_empty() && slot.enchantments.is_none() { None } else { Some(levels) };
    Ok(())
}

/// The stack's attribute modifiers, from AttributesModifierHook.getHeldAttributeModifiers and ModifiableArmorItem: a melee weapon's
/// attack damage is its stat and its attack speed the stat less the player's base of 4 (main hand); an armor piece's armor, toughness and
/// knockback resistance are its stats (its own slot); then each modifier's attribute modules add theirs. A broken tool has none. The
/// weapon and armor checks need the item tags, so without them (or for another tool) the item's own modifiers stay.
fn attribute_modifiers(data: &pluto_lib::NbtCompound, env: &Environment) -> Option<Vec<pluto_lib::NbtCompound>> {
    use pluto_lib::Nbt;
    let (item, tags) = (env.item?, env.item_tags?);
    let in_tag = |name: &str| Identifier::new("tconstruct", name).ok().and_then(|tag| tags.get_tag(&tag)).is_some_and(|members| members.contains(item));
    let melee = in_tag("modifiable/melee/weapon");
    let armor_slot = if in_tag("modifiable/armor") {
        let path = item.path();
        if path.ends_with("_helmet") { Some("head") } else if path.ends_with("_chestplate") { Some("chest") }
        else if path.ends_with("_leggings") { Some("legs") } else if path.ends_with("_boots") { Some("feet") } else { None }
    } else { None };
    if !melee && armor_slot.is_none() { return None; }
    if is_broken(data) { return Some(Vec::new()); }
    let modifier = |attribute: &str, id: String, amount: f64, slot: &str| {
        let mut entry = pluto_lib::NbtCompound::new();
        entry.insert("type".into(), Nbt::String(format!("minecraft:{attribute}")));
        entry.insert("id".into(), Nbt::String(id));
        entry.insert("amount".into(), Nbt::Double(amount));
        entry.insert("operation".into(), Nbt::String("add_value".into()));
        entry.insert("slot".into(), Nbt::String(slot.to_owned()));
        entry
    };
    let mut list = Vec::new();
    let mut wanted = Vec::new();
    if melee {
        wanted.push("mainhand");
        list.push(modifier("attack_damage", "tconstruct:tool_attack_damage".into(), f64::from(stat(data, Stat::AttackDamage)), "mainhand"));
        // The tool's attack speed counts from the player's base of 4.
        list.push(modifier("attack_speed", "tconstruct:tool_attack_speed".into(), f64::from(stat(data, Stat::AttackSpeed)) - 4.0, "mainhand"));
    }
    if let Some(slot) = armor_slot {
        wanted.push(slot);
        for (attribute, value) in [("armor", stat(data, Stat::Armor)), ("armor_toughness", stat(data, Stat::ArmorToughness)), ("knockback_resistance", stat(data, Stat::KnockbackResistance))] {
            if value > 0.0 { list.push(modifier(attribute, format!("tconstruct:armor_{attribute}/{slot}"), f64::from(value), slot)); }
        }
    }
    list.extend(crate::tool_stack::attribute_entries(data, env, &wanted));
    Some(list)
}

/// ToolStack.rebuildStats on the slot's data followed by the visual mirror. `Ok(false)` means the
/// environment was not ready and nothing changed.
pub(crate) fn rebuild(slot: &mut ItemSlot, env: &Environment) -> Result<bool, String> {
    if !rebuild_stats(&mut slot.custom_data, env)? { return Ok(false); }
    sync_visuals(slot, env)?;
    Ok(true)
}

/// ToolDamageUtil.damage on a stack. Returns the damage actually applied and whether it broke.
pub(crate) fn damage_slot(
    slot: &mut ItemSlot, env: &Environment, amount: i32, consumes_resources: bool, has_durability_tag: bool,
    cause: Option<&pluto_lib::Identifier>, random: &mut dyn FnMut() -> f32,
) -> Result<tool_damage::Damaged, String> {
    // ModifiableItem.verifyTag runs on a stack's first inventory tick, so a worn stack that was never built (a command-given piece,
    // an old save) is built before it takes wear; without it the missing durability reads as 1 and the first hit would break it.
    if amount > 0 && !is_initialized(&slot.custom_data) && !rebuild(slot, env)? { return Ok(tool_damage::Damaged { applied: 0, broke: false }); }
    let before = slot.custom_data.clone();
    let outcome = tool_damage::damage_tool_reduced(&mut slot.custom_data, amount, consumes_resources, has_durability_tag,
        // Overslime soaks damage before any other reduction.
        &mut |nbt, amount| { let amount = crate::capacity::absorb(nbt, env, cause, amount, random); crate::tool_stack::reduce_damage(nbt, env, cause, amount, random) });
    // The shield (persistent data) can change without any wear landing, and the attributes that follow it must follow.
    if outcome.applied > 0 || slot.custom_data != before { sync_visuals(slot, env)?; }
    Ok(outcome)
}

/// ToolDamageUtil.repair on a stack; returns the amount repaired.
pub(crate) fn repair_slot(slot: &mut ItemSlot, env: &Environment, amount: i32) -> Result<i32, String> {
    let repaired = tool_damage::repair(&mut slot.custom_data, amount);
    if repaired > 0 { sync_visuals(slot, env)?; }
    Ok(repaired)
}

/// Remaining durability, 0 while broken.
pub(crate) fn remaining(slot: &ItemSlot) -> i32 { current_durability(&slot.custom_data) }
pub(crate) fn max(slot: &ItemSlot) -> i32 { durability(&slot.custom_data) }
pub(crate) fn broken(slot: &ItemSlot) -> bool { is_broken(&slot.custom_data) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{materials::Definitions as Materials, modifiers::Registry, tool_definitions::Data};
    use pluto_lib::{Identifier, Nbt};
    use serde_json::json;
    struct Always;
    impl crate::materials::Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &serde_json::Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }
    fn world() -> (Data, Materials, Registry) {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:base_stats","stats":{"tconstruct:durability":10,"tconstruct:mining_speed":4,"tconstruct:harvest_tier":"minecraft:stone"}},
            {"type":"tconstruct:is_effective","predicate_type":"mantle:tag","tag":"minecraft:mineable/pickaxe"}
        ]})).unwrap();
        let mut materials = Materials::default();
        materials.server_loaded = true;
        let mut registry = Registry::default();
        registry.reload(&[], &Always);
        (data, materials, registry)
    }
    #[test]
    fn rebuilding_a_slot_publishes_the_bar_and_mining_component_and_damage_keeps_them_in_step() {
        let (data, materials, registry) = world();
        let env = Environment { definition: Some(&data), has_materials: false, materials: &materials, modifiers: &registry, item_tags_loaded: true, item: None, item_tags: None };
        let mut slot = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        assert!(rebuild(&mut slot, &env).unwrap());
        assert_eq!((slot.max_damage, slot.damage, max(&slot), remaining(&slot)), (Some(10), 0, 10, 10));
        let tool = slot.tool.clone().expect("mining component");
        assert_eq!(tool["default_mining_speed"], Nbt::Float(1.0));
        // Damage moves the bar; the slot carries it without touching the item's own max.
        let hit = damage_slot(&mut slot, &env, 4, true, true, None, &mut || 1.0).unwrap();
        assert_eq!((hit.applied, hit.broke, slot.damage, remaining(&slot)), (4, false, 4, 6));
        assert_eq!(slot.tool, Some(tool), "mining is unchanged until the tool breaks");
        // Breaking raises the fake max by one and slows mining to the broken speed.
        let hit = damage_slot(&mut slot, &env, 100, true, true, None, &mut || 1.0).unwrap();
        assert!(hit.broke && broken(&slot));
        assert_eq!((slot.max_damage, slot.damage, remaining(&slot)), (Some(11), 10, 0));
        assert_eq!(slot.tool.as_ref().unwrap()["default_mining_speed"], Nbt::Float(0.3));
        // Repair revives it and restores the normal component.
        assert_eq!(repair_slot(&mut slot, &env, 3).unwrap(), 3);
        assert!(!broken(&slot));
        assert_eq!((slot.max_damage, slot.damage), (Some(10), 7));
        assert_eq!(slot.tool.as_ref().unwrap()["default_mining_speed"], Nbt::Float(1.0));
        // No-op damage (creative) leaves everything alone.
        let before = slot.clone();
        assert_eq!(damage_slot(&mut slot, &env, 2, false, true, None, &mut || 1.0).unwrap().applied, 0);
        assert_eq!(slot, before);
    }
    #[test]
    fn an_unready_environment_leaves_the_slot_untouched() {
        let (data, materials, registry) = world();
        let env = Environment { definition: Some(&data), has_materials: false, materials: &materials, modifiers: &registry, item_tags_loaded: false, item: None, item_tags: None };
        let mut slot = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        assert!(!rebuild(&mut slot, &env).unwrap());
        assert_eq!((slot.max_damage, slot.damage, slot.tool.is_none(), slot.custom_data.is_empty()), (None, 0, true, true));
    }
}
