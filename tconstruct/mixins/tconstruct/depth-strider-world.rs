use horizon_items::items::ItemStack;
use horizon_client::enchant::Enchantments;

const ENCHANTMENT: &str = "minecraft:depth_strider";
const ATTRIBUTE: &str = "minecraft:water_movement_efficiency";
const MODIFIER: &str = "tconstruct:depth_strider/feet";

// Use the same native enchantment formula as ordinary boots. Subtract its
// existing contribution so an already enchanted tool is not counted twice.
fn native_delta(enchantments: &Enchantments, stack: &ItemStack, level: i32) -> Option<f64> {
    let definition = enchantments.get(ENCHANTMENT)?;
    if definition.slots.len() != 1 || definition.slots[0] != "feet" { return None; }
    let effects = definition.effects.as_object()?;
    if effects.len() != 1 { return None; }
    let attributes = effects.get("minecraft:attributes")?.as_array()?;
    if attributes.len() != 1 { return None; }
    let existing = stack.enchantment_level(ENCHANTMENT);
    if existing > 3 { return None; }
    let combined = existing.checked_add(u32::try_from(level).ok()?)?.min(3);
    let amount = |level| -> Option<f64> {
        if level == 0 { return Some(0.0); }
        let mut probe = ItemStack::new(&stack.item, 1);
        probe.enchant(ENCHANTMENT, level);
        let rows = enchantments.attributes(&probe);
        let (attribute, _, value, operation) = rows.first()?;
        (rows.len() == 1 && attribute == ATTRIBUTE && operation == "add_value"
            && value.is_finite() && (0.0..=1.0).contains(value)).then_some(*value)
    };
    let delta = amount(combined)? - amount(existing)?;
    (delta.is_finite() && delta >= 0.0).then_some(delta)
}

pub(super) fn refresh(game: &mut super::Game<'_>) {
    // Server-owned attributes are synchronized to remote players. Never add a
    // second local projection on top of the authoritative value.
    if game.remote.is_some() { return; }
    let wanted = (|| {
        let stack = game.armor[3].as_ref()?;
        let level = super::tconstruct_source_depth_strider::level(&game.world, Some(stack))?;
        native_delta(&game.stations().enchantments, stack, level).filter(|amount| *amount > 0.0)
    })();
    let Some(player) = game.player.as_mut() else { return; };
    let key = player.attributes.key(ATTRIBUTE);
    let Ok(present) = player.attributes.has_modifier(key.clone(), MODIFIER) else { return; };
    if !present && wanted.is_none() { return; }
    if present && wanted.is_some() && player.attributes.modifier_value(key, MODIFIER).ok() == wanted { return; }
    let _ = player.set_modifier(ATTRIBUTE, MODIFIER, wanted.unwrap_or(0.0), "add_value", wanted.is_some());
}
