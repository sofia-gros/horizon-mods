use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use super::tconstruct_fire_duration as rule;

const ID: &str = "tconstruct:fire_protection";
const ENCHANTMENT: &str = "minecraft:fire_protection";
const ATTRIBUTE: &str = "minecraft:burning_time";

fn level(game: &super::Game<'_>, stack: &ItemStack, fire_supported: bool, scorch_supported: bool) -> Option<i32> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    // The source enchantment hook itself has no brokenness exclusion.
    if !matches!(custom.get("tic_broken"), None | Some(Tag::Byte(0)) | Some(Tag::Byte(1))) { return None; }
    let fire = if fire_supported {
        let raw = match custom.get("tic_upgrades") {
            None => 0,
            Some(upgrades @ Tag::Compound(_)) => match upgrades.get(ID) {
                None => 0, Some(Tag::Int(raw)) => *raw, _ => return None,
            },
            _ => return None,
        };
        let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
        rule::enchantment_level(traits.levels.get(ID).copied().unwrap_or(0), raw)?
    } else { 0 };
    let scorch = if scorch_supported { super::tconstruct_scorch_protection_world::level(game,stack)? } else { 0 };
    super::tconstruct_source_scorch_protection::combined_enchantment(fire,scorch)
}

fn wanted(game: &super::Game<'_>) -> Option<(i32, f64)> {
    let fire_supported = super::tconstruct_source_protection::supported_definition(&game.world, ID);
    let scorch_supported = super::tconstruct_scorch_protection_world::supported(game);
    if !fire_supported && !scorch_supported { return None; }
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let worn = tags.get("tconstruct:modifiable/armor/worn")?;
    let enchantments = &game.stations().enchantments;
    let mut maximum = 0;
    let mut native_amounts = Vec::new();
    let mut has_modifier = false;
    for (slot, stack) in game.armor.iter().enumerate().filter_map(|(i,s)| s.as_ref().map(|s| (i,s))) {
        let existing = stack.enchantment_level(ENCHANTMENT);
        let source = if rule::armor_slot(&stack.item) == Some(slot) && worn.contains(stack.item.as_ref()) {
            level(game, stack, fire_supported, scorch_supported)?
        } else { 0 };
        has_modifier |= source > 0;
        maximum = maximum.max(i32::try_from(existing).ok()?.checked_add(source)?);
        if existing > 0 {
            let mut probe = ItemStack::new(&stack.item, 1);
            probe.enchant(ENCHANTMENT, existing);
            let rows = enchantments.attributes(&probe);
            let (attribute, _, amount, operation) = rows.first()?;
            let expected = f64::from(-0.15f32 + -0.15f32 * existing.saturating_sub(1) as f32);
            if rows.len() != 1 || attribute != ATTRIBUTE || operation != "add_multiplied_base"
                || *amount != expected { return None; }
            native_amounts.push(*amount);
        }
    }
    // Do not change vanilla-only gear. The duration/contact helpers retain
    // native behavior when its observed result or multiplier differs.
    has_modifier.then_some((maximum, native_amounts.into_iter().fold(1.0f64, |n, a| n + a).max(0.0)))
}

pub(super) fn refresh(game: &mut super::Game<'_>) {
    let context = if game.remote.is_none() { wanted(game) } else { None };
    if let Some(player) = game.player.as_mut() {
        *player.status.__horizon_mixins_tconstruct_fire_duration_context_mut() = context;
    }
}
