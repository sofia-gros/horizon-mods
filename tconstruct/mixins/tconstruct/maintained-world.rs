use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
const ID: &str = "tconstruct:maintained";
const ATTRIBUTE: &str = "minecraft:armor_toughness";
const SLOTS: [&str;6] = ["head","chest","legs","feet","mainhand","offhand"];

fn supported(game: &super::Game<'_>) -> bool {
    game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw|raw.as_ref().ok())
        .and_then(|raw|raw.0.get("tconstruct:tinkering/modifiers/maintained"))
        .and_then(|layers|layers.last())
        .and_then(|bytes|serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some_and(|value|super::tconstruct_maintained::supported_value(&value))
}

fn contribution(game: &super::Game<'_>, stack: &ItemStack) -> Option<f64> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let maximum = match custom.get("tic_max_damage") { Some(Tag::Int(n)) if *n > 0 => *n as u32, _ => return None };
    let broken = match custom.get("tic_broken") { None | Some(Tag::Byte(0)) => false, Some(Tag::Byte(1)) => true, _ => return None };
    if broken || stack.damage >= maximum { return Some(0.0); }
    let traits = super::tconstruct_source_material_traits::levels(&game.world,stack)?;
    let level = super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,ID)?;
    if level == 0 { return Some(0.0); }
    // Reuse the supported rebuild baseline, keeping its float maximum rather
    // than approximating the source denominator with saved integer max_damage.
    for id in ["tconstruct:preserved","tconstruct:ductile","tconstruct:emerald","tconstruct:netherite"] {
        super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,id)?;
    }
    let diamond = super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,"tconstruct:diamond")? > 0;
    // The current shared rebuild supports Overlord only as a material trait.
    let overlord = super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,"tconstruct:overlord")?;
    if overlord != traits.levels.get("tconstruct:overlord").copied().unwrap_or(0) { return None; }
    let stats = &traits.base_stats;
    let factor = super::tconstruct_source_material_effects::ordered_durability_multiplier(stack,stats,&traits.levels,true);
    let raw = ((stats.durability_base_raw + if diamond {250.0} else {0.0})
        * (1.0 + stats.durability_percent_raw) * factor).clamp(1.0,i32::MAX as f32);
    super::tconstruct_maintained::toughness(raw,maximum,stack.damage,level,false)
}

fn wanted(game: &super::Game<'_>) -> Option<[f64;6]> {
    if !supported(game) { return None; }
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let armor = tags.get("tconstruct:modifiable/armor")?;
    let hands = [game.inventory.get(game.selected).and_then(Option::as_ref),game.offhand.as_ref()];
    let mut values = [0.0;6];
    for (index,stack) in game.armor.iter().map(Option::as_ref).chain(hands).enumerate() {
        let Some(stack) = stack else { continue; };
        let eligible = if index < 4 {
            super::tconstruct_fire_duration::armor_slot(&stack.item) == Some(index)
        } else {
            matches!(stack.item.as_ref(),"tconstruct:travelers_shield" | "tconstruct:plate_shield" | "tconstruct:battlesign")
                && tags.get("tconstruct:modifiable/held").is_some_and(|items|items.contains(stack.item.as_ref()))
        };
        if eligible && armor.contains(stack.item.as_ref()) {
            values[index] = contribution(game,stack)?;
        }
    }
    Some(values)
}

pub(super) fn refresh(game: &mut super::Game<'_>) {
    // A remote client receives the server attribute map, including these IDs.
    if game.remote.is_some() { return; }
    let values = wanted(game).unwrap_or([0.0;6]);
    let Some(player) = game.player.as_mut() else { return; };
    for (slot,amount) in SLOTS.into_iter().zip(values) {
        // Owned IDs are replaced, never folded into the base or other mods'
        // attributes. Repeated wear/status refreshes cannot accumulate bonuses.
        let id = format!("tconstruct:maintained_toughness/{slot}");
        let key = player.attributes.key(ATTRIBUTE);
        let Ok(present) = player.attributes.has_modifier(key.clone(),&id) else { continue; };
        if !present && amount == 0.0 { continue; }
        if present && amount > 0.0 && player.attributes.modifier_value(key,&id).ok() == Some(amount) { continue; }
        let _ = player.set_modifier(ATTRIBUTE,&id,amount,"add_value",amount > 0.0);
    }
}
