use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

const ID: &str = "tconstruct:projectile_protection";
const ATTRIBUTE: &str = "minecraft:knockback_resistance";
const MODIFIER: &str = "tconstruct:projectile_protection/maximum";

fn armor_slot(item: &str) -> Option<usize> {
    match item {
        "tconstruct:travelers_helmet" | "tconstruct:plate_helmet" | "tconstruct:slime_helmet" => Some(0),
        "tconstruct:travelers_chestplate" | "tconstruct:plate_chestplate" | "tconstruct:slimy_chestplate" | "tconstruct:slime_wings" => Some(1),
        "tconstruct:travelers_leggings" | "tconstruct:plate_leggings" | "tconstruct:slime_leggings" => Some(2),
        "tconstruct:travelers_boots" | "tconstruct:plate_boots" | "tconstruct:slime_boots" => Some(3),
        _ => None,
    }
}

fn level(game: &super::Game<'_>, stack: &ItemStack) -> Option<f32> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom=stack.data()?.other("minecraft:custom_data")?;
    let maximum=match custom.get("tic_max_damage") { Some(Tag::Int(n)) if *n > 0 => *n as u32, _ => return None };
    if stack.damage >= maximum || !matches!(custom.get("tic_broken"),None | Some(Tag::Byte(0))) { return None; }
    let raw=match custom.get("tic_upgrades") {
        None => 0,
        Some(upgrades @ Tag::Compound(_)) => match upgrades.get(ID) {
            None => 0, Some(Tag::Int(raw)) => *raw, _ => return None,
        },
        _ => return None,
    };
    let traits=super::tconstruct_source_material_traits::levels(&game.world,stack)?;
    super::tconstruct_source_protection::effective_level(traits.levels.get(ID).copied().unwrap_or(0),raw)
}

fn wanted(game: &super::Game<'_>) -> Option<f64> {
    if !super::tconstruct_source_protection::supported_definition(&game.world,ID) { return None; }
    let tags=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let belongs=|stack:&ItemStack,tag:&str| tags.get(tag).is_some_and(|items|items.contains(stack.item.as_ref()));
    let mut levels=Vec::new();
    for (slot,stack) in game.armor.iter().enumerate().filter_map(|(slot,stack)|stack.as_ref().map(|stack|(slot,stack))) {
        if armor_slot(&stack.item) == Some(slot) && belongs(stack,"tconstruct:modifiable/armor/worn") {
            if let Some(level)=level(game,stack) { levels.push(level); }
        }
    }
    // Source held_tag applies in both hands without requiring active blocking.
    // Use the selected inventory hand, not a temporary item-use acting hand.
    for stack in [game.inventory.get(game.selected).and_then(Option::as_ref),game.offhand.as_ref()].into_iter().flatten() {
        if matches!(stack.item.as_ref(),"tconstruct:travelers_shield" | "tconstruct:plate_shield" | "tconstruct:battlesign")
            && belongs(stack,"tconstruct:modifiable/held") {
            if let Some(level)=level(game,stack) { levels.push(level); }
        }
    }
    super::tconstruct_source_protection::projectile_attribute(&levels)
}

pub(super) fn refresh(game: &mut super::Game<'_>) {
    if game.remote.is_some() { return; }
    let wanted=wanted(game);
    let Some(player)=game.player.as_mut() else { return; };
    let key=player.attributes.key(ATTRIBUTE);
    let Ok(present)=player.attributes.has_modifier(key.clone(),MODIFIER) else { return; };
    if !present && wanted.is_none() { return; }
    if present && wanted.is_some() && player.attributes.modifier_value(key,MODIFIER).ok() == wanted { return; }
    let _=player.set_modifier(ATTRIBUTE,MODIFIER,wanted.unwrap_or(0.0),"add_value",wanted.is_some());
}
