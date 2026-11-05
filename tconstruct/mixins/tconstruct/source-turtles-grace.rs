// Turtle's Grace contributes a virtual Respiration enchantment. Its separate
// swim attribute is deliberately not projected by this module.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use super::tconstruct_world::World;
const ID: &str = "tconstruct:turtles_grace";

pub(super) fn supported_value(value: &serde_json::Value) -> bool {
    value == &serde_json::json!({"level_display":"tconstruct:single_level","modules":[
        {"type":"tconstruct:attribute","attribute":"forge:swim_speed","each_level":0.1,
         "operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"]},
        {"type":"tconstruct:constant_enchantment","level":1,"name":"minecraft:respiration"}
    ]})
}

fn supported(world: &World) -> bool {
    world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw|raw.as_ref().ok())
        .and_then(|raw|raw.0.get("tconstruct:tinkering/modifiers/turtles_grace"))
        .and_then(|layers|layers.last())
        .and_then(|bytes|serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some_and(|value|supported_value(&value))
}

fn armor_slot(item: &str) -> Option<usize> {
    match item {
        "tconstruct:travelers_helmet" | "tconstruct:plate_helmet" | "tconstruct:slime_helmet" => Some(0),
        "tconstruct:travelers_chestplate" | "tconstruct:plate_chestplate" | "tconstruct:slimy_chestplate" | "tconstruct:slime_wings" => Some(1),
        "tconstruct:travelers_leggings" | "tconstruct:plate_leggings" | "tconstruct:slime_leggings" => Some(2),
        "tconstruct:travelers_boots" | "tconstruct:plate_boots" | "tconstruct:slime_boots" => Some(3),
        _ => None,
    }
}

fn level(world: &World, stack: &ItemStack) -> Option<i32> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    if !matches!(custom.get("tic_max_damage"),Some(Tag::Int(max)) if *max > 0)
        || !matches!(custom.get("tic_broken"),None | Some(Tag::Byte(0 | 1))) { return None; }
    // EnchantmentModifierHook queries every modifier even on a broken tool.
    // Unlike the separate attribute module, Constant has no broken-tool gate.
    let traits = super::tconstruct_source_material_traits::levels(world,stack)?;
    super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,ID)
}

/// Source Respiration picks the strongest armor enchantment result. Preserve
/// the existing head bonus and add only the amount Turtle's Grace contributes
/// above that baseline, including additive hooks on the same head item.
pub(super) fn extra(head_native: i32, head_respiration: i32, grace: [i32;4]) -> Option<i32> {
    if head_native < 0 || head_respiration < 0 || grace.iter().any(|n|*n < 0) { return None; }
    let baseline = head_native.checked_add(head_respiration)?;
    let head = baseline.checked_add(grace[0])?;
    let strongest = grace[1..].iter().copied().fold(head,i32::max);
    strongest.checked_sub(baseline)
}

pub(super) fn bonus(world: &World, armor: &[Option<ItemStack>;4]) -> Option<i32> {
    if !supported(world) { return None; }
    let tags = world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let worn = tags.get("tconstruct:modifiable/armor/worn")?;
    let mut grace = [0;4];
    let mut head_native = 0;
    for (slot,stack) in armor.iter().enumerate().filter_map(|(slot,stack)|stack.as_ref().map(|stack|(slot,stack))) {
        let native = i32::try_from(stack.enchantment_level("minecraft:respiration")).ok()?;
        // Out-of-slot command enchantments have modern host attribute behavior
        // outside this bounded projection. Never add another guessed bonus.
        if slot > 0 && native > 0 { return None; }
        if slot == 0 { head_native = native; }
        if armor_slot(&stack.item) == Some(slot) && worn.contains(stack.item.as_ref()) {
            grace[slot] = level(world,stack)?;
        }
    }
    let head_respiration = super::tconstruct_source_respiration::level(world,armor[0].as_ref()).unwrap_or(0);
    extra(head_native,head_respiration,grace).filter(|n|*n > 0)
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn worn_copies_take_maximum_but_same_head_enchantments_add() {
        assert_eq!(extra(0,0,[0,1,1,1]),Some(1));
        assert_eq!(extra(3,0,[0,1,0,0]),Some(0));
        assert_eq!(extra(1,1,[0,3,0,0]),Some(1));
        assert_eq!(extra(1,1,[2,3,0,0]),Some(2));
        assert_eq!(extra(0,0,[0;4]),Some(0));
    }
    #[test] fn invalid_or_overflowing_inputs_fail_closed() {
        assert_eq!(extra(-1,0,[0;4]),None);
        assert_eq!(extra(0,0,[0,-1,0,0]),None);
        assert_eq!(extra(i32::MAX,1,[0;4]),None);
        assert_eq!(extra(i32::MAX,0,[1,0,0,0]),None);
    }
    #[test] fn exact_definition_includes_both_source_modules() {
        let mut value = serde_json::json!({"level_display":"tconstruct:single_level","modules":[
            {"type":"tconstruct:attribute","attribute":"forge:swim_speed","each_level":0.1,
             "operation":"multiply_total","slots":["mainhand","offhand","feet","legs","chest","head"]},
            {"type":"tconstruct:constant_enchantment","level":1,"name":"minecraft:respiration"}
        ]});
        assert!(supported_value(&value));
        value["modules"][1]["level"] = serde_json::json!(2);
        assert!(!supported_value(&value));
    }
}
