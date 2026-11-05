// Bounded main-hand Knockback: Sword/Cleaver only; projectile and armor are separate.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use super::tconstruct_world::World;

const STOCK: &str = r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:knockback","allow":"projectile","each_level":0.5},{"type":"tconstruct:attribute","attribute":"minecraft:generic.attack_knockback","each_level":1,"operation":"addition","slots":["mainhand","feet","legs","chest","head"]},{"type":"tconstruct:sling_force","formula":[0.25,"$level","*","$multiplier","*",1,"$knockback_resistance","-","*","$knockback_multiplier","*","$value","+"],"percent":false,"sling":{"type":"tconstruct:tag","tag":"tconstruct:knockback_slings/self"},"variables":{"knockback_multiplier":{"type":"tconstruct:entity","attribute":"tconstruct:generic.knockback_multiplier","entity_type":"tconstruct:attribute","fallback":1},"knockback_resistance":{"type":"tconstruct:entity","attribute":"minecraft:generic.knockback_resistance","entity_type":"tconstruct:attribute","fallback":0}}},{"type":"tconstruct:sling_force","formula":[0.25,"$level","*","$multiplier","*","$value","+"],"percent":false,"sling":{"type":"tconstruct:tag","tag":"tconstruct:knockback_slings/target"}}]}"#;

pub(super) fn supported_definition(world: &World) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref().and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes) = raw.0.get("tconstruct:tinkering/modifiers/knockback")
        .and_then(|layers| layers.last()) else { return false; };
    matches!((serde_json::from_slice::<serde_json::Value>(bytes), serde_json::from_str::<serde_json::Value>(STOCK)),
        (Ok(actual), Ok(expected)) if actual == expected)
}

pub(super) fn level(world: &World, held: Option<&ItemStack>) -> Option<i32> {
    let stack = held?;
    if stack.count != 1 || stack.wire_patch().is_some()
        || !matches!(stack.item.as_ref(), "tconstruct:sword" | "tconstruct:cleaver")
        || !supported_definition(world) { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    if let Some(upgrades) = custom.get("tic_upgrades") {
        let Tag::Compound(_) = upgrades else { return None; };
        match upgrades.get("tconstruct:knockback") {
            None => {},
            Some(Tag::Int(level)) if (0..=3).contains(level) => {},
            _ => return None,
        }
    }
    let max = match custom.get("tic_max_damage") {
        Some(Tag::Int(max)) if *max > 0 => *max as u32,
        _ => return None,
    };
    if stack.damage >= max || !matches!(custom.get("tic_broken"), None | Some(Tag::Byte(0))) { return None; }
    let tags = world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    if !tags.get("tconstruct:modifiable/melee")?.contains(stack.item.as_ref()) { return None; }
    let traits = super::tconstruct_source_material_traits::levels(world, stack)?;
    if super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, "tconstruct:no_melee")? != 0 { return None; }
    let level = super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, "tconstruct:knockback")?;
    if !(1..=i16::MAX as i32).contains(&level) { return None; }
    // Other source knockback modules require ordered dispatch. Do not claim
    // additive parity for a tool carrying a reduction/replacement module.
    let mut ids: std::collections::BTreeSet<String> = traits.levels.keys().cloned().collect();
    if let Some(saved) = custom.get("tic_upgrades") {
        let Tag::Compound(entries) = saved else { return None; };
        ids.extend(entries.keys().map(|id| id.to_string_lossy().to_string()));
    }
    let raw = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()?.as_ref().ok()?;
    for id in ids {
        if id == "tconstruct:knockback"
            || super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, &id)? == 0 { continue; }
        let (namespace, name) = id.split_once(':')?;
        let bytes = raw.0.get(&format!("{namespace}:tinkering/modifiers/{name}"))?.last()?;
        let definition: serde_json::Value = serde_json::from_slice(bytes).ok()?;
        if definition.get("modules").and_then(serde_json::Value::as_array).is_some_and(|modules|
            modules.iter().any(|module| module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:knockback"))) { return None; }
    }
    Some(level)
}

