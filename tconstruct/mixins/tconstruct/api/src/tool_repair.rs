//! TinkerStationRepairRecipe matching and repair arithmetic from SlimeKnights (MIT; see LICENSE).
//! Pure over abstract input slots. Resolving a stack to a material (repair kit or material
//! recipe), shrinking real stacks and the modifier REPAIR_FACTOR hook are the caller's job;
//! no modifier defines that hook yet, so the factor is the identity.
use crate::{tool_damage, tool_stack::{damage, is_broken}};
use pluto_lib::{Identifier, NbtCompound};

/// MaterialRecipe.INGOTS_PER_REPAIR.
pub(crate) const INGOTS_PER_REPAIR: f32 = 3.0;

/// How one input slot converts a material's repair amount into durability per item.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Source {
    /// IRepairKitItem: amount * kit_amount / INGOTS_PER_REPAIR.
    Kit { kit_amount: f32 },
    /// MaterialRecipe.scaleRepair: value * amount / INGOTS_PER_REPAIR / needed.
    Recipe { value: i32, needed: i32 },
}

/// One non-empty input stack. `material` is `None` for MaterialId.UNKNOWN.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Slot { pub material: Option<Identifier>, pub source: Source, pub count: i32 }

fn per_item(amount: f32, source: &Source) -> f32 {
    if amount <= 0.0 { return 0.0; }
    match source {
        Source::Kit { kit_amount } => amount * kit_amount / INGOTS_PER_REPAIR,
        Source::Recipe { value, needed } => *value as f32 * amount / INGOTS_PER_REPAIR / *needed as f32,
    }
}

/// matches(): damaged, tagged tool; every non-empty slot a known material; one shared material
/// that the tool can repair with. Empty slots are simply absent from `slots`.
pub(crate) fn matches(
    tool: &NbtCompound, has_durability_tag: bool, slots: &[Slot], can_repair_with: impl Fn(&Identifier) -> bool,
) -> bool {
    if !has_durability_tag { return false; }
    // Undamaged tools defer to other recipes such as part swapping.
    if !is_broken(tool) && damage(tool) == 0 { return false; }
    let mut shared: Option<&Identifier> = None;
    for slot in slots {
        let Some(material) = &slot.material else { return false; };
        match shared {
            None => { if !can_repair_with(material) { return false; } shared = Some(material); }
            Some(first) if first != material => return false,
            Some(_) => {}
        }
    }
    shared.is_some()
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Plan {
    /// Items to consume from each slot, in slot order.
    pub consumed: Vec<i32>,
    /// Durability restored; may overshoot the need by the last item's rounding, repair() caps it.
    pub repaired: i32,
}

/// repairFromSlot: items needed (capped by the stack) and the whole durability they give.
fn from_slot(slot: &Slot, needed: i32, amount_for: &impl Fn(&Identifier) -> f32) -> (i32, i32) {
    let Some(material) = &slot.material else { return (0, 0); };
    let per = per_item(amount_for(material), &slot.source);
    // RepairFactorModifierHook.getRepairFactor is the identity until modifiers define it.
    if per <= 0.0 { return (0, 0); }
    let applied = slot.count.min((needed as f32 / per).ceil() as i32);
    (applied, (applied as f32 * per) as i32)
}

/// getValidatedResult's accounting. `None` means nothing would be repaired (RecipeResult.pass).
pub(crate) fn plan(damage_needed: i32, slots: &[Slot], amount_for: impl Fn(&Identifier) -> f32) -> Option<Plan> {
    let mut remaining = damage_needed;
    let mut consumed = vec![0; slots.len()];
    for (index, slot) in slots.iter().enumerate() {
        if remaining <= 0 { break; }
        let (applied, repaired) = from_slot(slot, remaining, &amount_for);
        consumed[index] = applied;
        remaining -= repaired;
    }
    (remaining < damage_needed).then(|| Plan { consumed, repaired: damage_needed - remaining })
}

/// The full recipe on a tool copy: returns the plan and leaves `tool` repaired by it.
pub(crate) fn apply(tool: &mut NbtCompound, slots: &[Slot], amount_for: impl Fn(&Identifier) -> f32) -> Option<Plan> {
    let plan = plan(damage(tool), slots, amount_for)?;
    tool_damage::repair(tool, plan.repaired);
    Some(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_stack::{current_durability, TAG_STATS};
    use pluto_lib::Nbt;
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn damaged(durability: f32, damage: i32) -> NbtCompound {
        let mut stats = NbtCompound::new();
        stats.insert("tconstruct:durability".into(), Nbt::Float(durability));
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_STATS.into(), Nbt::Compound(stats));
        tool_damage::damage_tool(&mut nbt, damage, true, true);
        nbt
    }
    fn kit(material: &str, count: i32) -> Slot { Slot { material: Some(id(material)), source: Source::Kit { kit_amount: 3.0 }, count } }
    fn ingot(material: &str, count: i32) -> Slot { Slot { material: Some(id(material)), source: Source::Recipe { value: 1, needed: 1 }, count } }
    #[test]
    fn items_are_rounded_up_and_the_overshoot_is_capped_by_repair() {
        // 100 damage, 35 per ingot-equivalent (value 1 / needed 1 / 3 ingots per repair -> 35/3 per item).
        let mut tool = damaged(200.0, 100);
        let plan = apply(&mut tool, &[ingot("test:wood", 64)], |_| 35.0).unwrap();
        // per item = 35/3 = 11.67; ceil(100/11.67) = 9 items restoring (int)(9*11.67) = 105.
        assert_eq!(plan, Plan { consumed: vec![9], repaired: 105 });
        assert_eq!(damage(&tool), 0, "repair() never goes past full");
        assert_eq!(current_durability(&tool), 200);
    }
    #[test]
    fn short_stacks_repair_partially_and_later_slots_continue() {
        let mut tool = damaged(200.0, 150);
        // Kit: 3 ingots worth -> amount * 3 / 3 = 30 per kit; two kits give 60.
        let plan = apply(&mut tool, &[kit("test:wood", 2), ingot("test:wood", 64)], |_| 30.0).unwrap();
        assert_eq!(plan.consumed[0], 2);
        // Remaining 90 at 10 per item needs exactly 9 items.
        assert_eq!(plan.consumed[1], 9);
        assert_eq!(plan.repaired, 150);
        assert_eq!(damage(&tool), 0);
        let mut tool = damaged(200.0, 150);
        let plan = apply(&mut tool, &[kit("test:wood", 2)], |_| 30.0).unwrap();
        assert_eq!((plan.consumed, plan.repaired, damage(&tool)), (vec![2], 60, 90));
    }
    #[test]
    fn zero_value_materials_pass_and_matching_enforces_one_known_material() {
        let tool = damaged(200.0, 50);
        let mut copy = tool.clone();
        assert!(apply(&mut copy, &[kit("test:stone", 4)], |_| 0.0).is_none());
        assert_eq!(damage(&copy), 50);
        let allow = |material: &Identifier| material.path() != "banned";
        assert!(matches(&tool, true, &[kit("test:wood", 1), ingot("test:wood", 1)], allow));
        assert!(!matches(&tool, true, &[kit("test:wood", 1), ingot("test:stone", 1)], allow), "mixed materials");
        assert!(!matches(&tool, true, &[kit("test:banned", 1)], allow), "first material must be repair-capable");
        assert!(!matches(&tool, true, &[Slot { material: None, source: Source::Kit { kit_amount: 3.0 }, count: 1 }], allow), "unknown material");
        assert!(!matches(&tool, true, &[], allow), "all slots empty");
        assert!(!matches(&tool, false, &[kit("test:wood", 1)], allow), "needs the durability tag");
        assert!(!matches(&damaged(200.0, 0), true, &[kit("test:wood", 1)], allow), "undamaged defers to other recipes");
        let mut broken = damaged(200.0, 0);
        tool_damage::break_tool(&mut broken);
        assert!(matches(&broken, true, &[kit("test:wood", 1)], allow), "broken tools count even with zero damage");
    }
}
