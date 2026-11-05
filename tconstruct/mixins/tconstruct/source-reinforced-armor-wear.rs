// Canonical definitions: Copyright (c) 2022 SlimeKnights, MIT.
// TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2.
// Full notice: tconstruct/third_party/tconstruct/LICENSE.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
const DENSE: &str = "tconstruct:dense";
const REINFORCED: &str = "tconstruct:reinforced";
const DEFINITIONS: &str = r###"{"reinforced":{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:reduce_tool_damage","cause":{"type":"mantle:inverted","inverted_type":"tconstruct:tag","tag":"tconstruct:durability/bypass_reinforced"},"formula":[0.025,"$level","*",11.0,"$level","-","*"],"modifier_level":{"max":5},"tool":{"type":"tconstruct:has_modifier","check":"all","level":0,"modifier":{"type":"tconstruct:single","modifier":"tconstruct:unbreakable"}}},{"type":"tconstruct:reduce_tool_damage","cause":{"type":"mantle:inverted","inverted_type":"tconstruct:tag","tag":"tconstruct:durability/bypass_reinforced"},"each_level":0.05,"flat":0.5,"modifier_level":{"min":6},"tool":{"type":"tconstruct:has_modifier","check":"all","level":0,"modifier":{"type":"tconstruct:single","modifier":"tconstruct:unbreakable"}}}]},"unbreakable":{"level_display":"tconstruct:no_levels","modules":[{"type":"tconstruct:requirements","display_modifiers":[{"level":1,"name":"tconstruct:netherite"},{"level":5,"name":"tconstruct:reinforced"}],"requirement":{"type":"mantle:and","predicates":[{"type":"tconstruct:has_modifier","check":"all","modifier":{"type":"tconstruct:single","modifier":"tconstruct:netherite"}},{"type":"tconstruct:has_modifier","check":"all","level":{"min":5},"modifier":{"type":"tconstruct:single","modifier":"tconstruct:reinforced"}}]},"translation_key":"modifier.tconstruct.unbreakable.requirements"},{"type":"tconstruct:durability_color","color":"FFFFFF"},{"type":"tconstruct:reduce_tool_damage","cause":{"type":"mantle:inverted","inverted_type":"tconstruct:tag","tag":"tconstruct:durability/bypass_reinforced"},"flat":1.0}],"priority":125}}"###;

pub(super) fn supported(id: &str, value: &serde_json::Value) -> bool {
    static EXPECTED: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    let expected = EXPECTED.get_or_init(|| serde_json::from_str(DEFINITIONS).expect("retained prevention definitions"));
    id.strip_prefix("tconstruct:").and_then(|id| expected.get(id)).is_some_and(|expected| expected == value)
}

fn upgrade(stack: &ItemStack, id: &str) -> Option<i32> {
    match stack.data()?.other("minecraft:custom_data")?.get("tic_upgrades") {
        None => Some(0),
        Some(upgrades @ Tag::Compound(_)) => match upgrades.get(id) {
            None => Some(0), Some(Tag::Int(n)) if *n >= 0 => Some(*n), _ => None,
        },
        _ => None,
    }
}

pub(super) fn chance(trait_level: i32, progress: i32) -> Option<f32> {
    if trait_level < 0 || progress < 0 { return None; }
    let remainder = progress % 4;
    let complete = trait_level.checked_add(progress / 4)?;
    let logical = complete.checked_add(i32::from(remainder != 0))?;
    let effective = if remainder == 0 { logical as f32 }
        else { logical as f32 + (remainder - 4) as f32 / 4.0f32 };
    Some(if logical <= 5 { (0.025f32 * effective) * (11.0f32 - effective) }
        else { 0.5f32 + 0.05f32 * effective })
}

/// ModifierNBT.Builder preserves insertion order for equal priority. Upgrades
/// are added before tool/material traits; merging retains the first position.
fn dense_first(stack: &ItemStack, trait_order: &[String], dense_upgrade: i32, reinforced_upgrade: i32) -> Option<bool> {
    let mut order = Vec::new();
    if dense_upgrade > 0 && reinforced_upgrade > 0 {
        let Some(Tag::List(saved)) = stack.data()?.other("minecraft:custom_data")?.get("tic_upgrade_order") else { return None; };
        for id in saved {
            let Tag::String(id) = id else { return None; };
            let id = id.to_string().ok()?;
            if id == DENSE || id == REINFORCED {
                if order.contains(&id) { return None; }
                order.push(id);
            }
        }
        if order.len() != 2 { return None; }
    } else if dense_upgrade > 0 { order.push(DENSE.to_owned()); }
    else if reinforced_upgrade > 0 { order.push(REINFORCED.to_owned()); }
    for id in trait_order {
        if (id == DENSE || id == REINFORCED) && !order.contains(id) { order.push(id.clone()); }
    }
    if order.len() != 2 { return None; }
    Some(order[0] == DENSE)
}

pub(super) struct Plan {
    dense: i32,
    reinforced: f32,
    unbreakable: bool,
    bypass: bool,
    dense_first: bool,
}

pub(super) fn plan(raw: &super::tconstruct_modifier_snapshot::Raw, stack: &ItemStack,
    traits: &super::tconstruct_source_material_traits::Traits) -> Option<Plan> {
    let dense = super::tconstruct_source_material_traits::level_with_upgrade(stack, traits, DENSE)?;
    let unbreakable = super::tconstruct_source_material_traits::level_with_upgrade(stack, traits, "tconstruct:unbreakable")? > 0;
    let progress = upgrade(stack, REINFORCED)?;
    let reinforced = chance(traits.levels.get(REINFORCED).copied().unwrap_or(0), progress)?;
    let bypass = if unbreakable || reinforced > 0.0 {
        super::tconstruct_tanned_wear::member(raw, "tconstruct:durability/bypass_reinforced", "tconstruct:empty")?
    } else { false };
    let dense_first = if dense > 0 && reinforced > 0.0 && !unbreakable && !bypass {
        dense_first(stack, &traits.order, upgrade(stack, DENSE)?, progress)?
    } else { true };
    Some(Plan { dense, reinforced, unbreakable, bypass, dense_first })
}

fn roll(amount: u32, chance: f32, mut next: impl FnMut() -> f32) -> u32 {
    if amount == 0 || chance <= 0.0 { return amount; }
    if chance >= 1.0 { return 0; }
    (0..amount).filter(|_| next() >= chance).count() as u32
}

pub(super) fn reduce(amount: u32, plan: Plan, mut next: impl FnMut() -> f32) -> u32 {
    // Unbreakable priority125 precedes both default-priority100 modifiers.
    if plan.unbreakable && !plan.bypass { return 0; }
    // Reinforced's has_modifier(level=0) condition excludes it even when the
    // Unbreakable hook itself is bypassed. Dense has no such cause condition.
    let chance = if plan.unbreakable || plan.bypass { 0.0 } else { plan.reinforced };
    if plan.dense_first {
        let amount = super::tconstruct_dense_armor_wear::reduce(amount, plan.dense, &mut next);
        roll(amount, chance, next)
    } else {
        let amount = roll(amount, chance, &mut next);
        super::tconstruct_dense_armor_wear::reduce(amount, plan.dense, next)
    }
}

#[cfg(test)] mod tests {
    use super::*;
    fn stack(order: Option<&[&str]>) -> ItemStack {
        let mut item = ItemStack::new("tconstruct:plate_helmet", 1);
        let fields = match order {
            Some(ids) => Tag::compound([("tic_upgrade_order", Tag::List(ids.iter().map(|id| Tag::String((*id).into())).collect()))]),
            None => Tag::compound(Vec::<(&str, Tag)>::new()),
        };
        item.data_mut().set_other("minecraft:custom_data", Some(fields)); item
    }
    #[test] fn incremental_levels_merge_before_fractional_scaling() {
        assert_eq!(chance(0, 1), Some((0.025f32 * 0.25) * 10.75));
        assert_eq!(chance(2, 1), Some((0.025f32 * 2.25) * 8.75));
        assert_eq!(chance(0, 21), Some(0.5f32 + 0.05f32 * 5.25));
        assert_eq!(chance(-1, 1), None);
        assert_eq!(chance(i32::MAX, 4), None);
    }
    #[test] fn mixed_priorities_follow_first_insertion_not_alphabetical_id() {
        assert_eq!(dense_first(&stack(Some(&[REINFORCED, DENSE])), &[], 1, 4), Some(false));
        assert_eq!(dense_first(&stack(None), &[DENSE.into()], 0, 4), Some(false));
        assert_eq!(dense_first(&stack(None), &[REINFORCED.into(), DENSE.into()], 0, 0), Some(false));
        assert_eq!(dense_first(&stack(None), &[], 1, 4), None);
        assert_eq!(dense_first(&stack(Some(&[DENSE, DENSE])), &[], 1, 4), None);
    }
    #[test] fn unbreakable_bypass_disables_reinforced_but_not_dense() {
        let plan = Plan { dense: 1, reinforced: 0.75, unbreakable: true, bypass: false, dense_first: true };
        assert_eq!(reduce(4, plan, || panic!("Unbreakable stops before RNG")), 0);
        let plan = Plan { dense: 1, reinforced: 1.0, unbreakable: true, bypass: true, dense_first: true };
        let mut draws = 0;
        assert_eq!(reduce(4, plan, || { draws += 1; 0.9 }), 4);
        assert_eq!(draws, 4);
    }
    #[test] fn equal_priority_order_controls_remaining_point_draws() {
        let plan = Plan { dense: 1, reinforced: 1.0, unbreakable: false, bypass: false, dense_first: false };
        assert_eq!(reduce(4, plan, || panic!("Reinforced stops before Dense")), 0);
        let plan = Plan { dense: 1, reinforced: 1.0, unbreakable: false, bypass: false, dense_first: true };
        let mut draws = 0;
        assert_eq!(reduce(4, plan, || { draws += 1; 0.9 }), 0);
        assert_eq!(draws, 4);
    }
}
