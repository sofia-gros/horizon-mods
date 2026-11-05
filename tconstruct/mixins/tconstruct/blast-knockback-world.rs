use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

const ID: &str = "tconstruct:blast_protection";
const ENCHANTMENT: &str = "minecraft:blast_protection";
const ATTRIBUTE: &str = "minecraft:explosion_knockback_resistance";

fn level(game: &super::Game<'_>, stack: &ItemStack) -> Option<i32> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    // EnchantmentModule.Protection itself applies even on broken equipment.
    if !matches!(custom.get("tic_broken"), None | Some(Tag::Byte(0)) | Some(Tag::Byte(1))) { return None; }
    let raw = match custom.get("tic_upgrades") {
        None => 0,
        Some(upgrades @ Tag::Compound(_)) => match upgrades.get(ID) {
            None => 0, Some(Tag::Int(raw)) => *raw, _ => return None,
        },
        _ => return None,
    };
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    // Fire and Blast use the same LevelingInt enchantment module contract.
    super::tconstruct_fire_duration::enchantment_level(traits.levels.get(ID).copied().unwrap_or(0), raw)
}

fn wanted(game: &super::Game<'_>, native: f64) -> Option<f64> {
    if !super::tconstruct_source_protection::supported_definition(&game.world, ID) { return None; }
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let worn = tags.get("tconstruct:modifiable/armor/worn")?;
    let enchantments = &game.stations().enchantments;
    let mut pieces = Vec::with_capacity(4);
    let mut native_amounts = Vec::with_capacity(4);
    for (slot, stack) in game.armor.iter().enumerate().filter_map(|(i,s)| s.as_ref().map(|s| (i,s))) {
        let existing = stack.enchantment_level(ENCHANTMENT);
        let source = if super::tconstruct_fire_duration::armor_slot(&stack.item) == Some(slot)
            && worn.contains(stack.item.as_ref()) { level(game, stack)? } else { 0 };
        pieces.push((existing, source));
        if existing > 0 {
            let mut probe = ItemStack::new(&stack.item, 1);
            probe.enchant(ENCHANTMENT, existing);
            let rows = enchantments.attributes(&probe);
            let (attribute, _, amount, operation) = rows.first()?;
            let expected = f64::from(0.15f32 + 0.15f32 * existing.saturating_sub(1) as f32);
            if rows.len() != 1 || attribute != ATTRIBUTE || operation != "add_value"
                || *amount != expected { return None; }
            native_amounts.push(*amount);
        }
    }
    super::tconstruct_blast_knockback::replacement(native, &pieces, &native_amounts)
}

pub(super) fn resistance(game: &super::Game<'_>, native: f64) -> f64 {
    if game.remote.is_some() { return native; }
    wanted(game, native).unwrap_or(native)
}
