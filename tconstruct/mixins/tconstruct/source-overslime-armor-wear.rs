// Ordinary armor wear only. Overslime is a static source modifier at priority150,
// so a same-ID dynamic JSON file does not replace its registered behavior.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

pub(super) struct Plan { amount: u32, bypass: bool }

pub(super) fn stored(stack: &ItemStack, active: bool) -> Option<u32> {
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let amount = match custom.get("tic_overslime") {
        None => 0, Some(Tag::Int(n)) if *n >= 0 => *n as u32, _ => return None,
    };
    if !active { return (amount == 0).then_some(0); }
    // Keep this port's existing bounded saved-capacity contract. Do not invent
    // a capacity for old or malformed records, or spend orphan shield data.
    let capacity = match custom.get("tic_overslime_capacity") {
        Some(Tag::Int(n)) if (0..=32767).contains(n) => *n as u32,
        _ => return None,
    };
    (amount <= capacity).then_some(amount)
}

pub(super) fn plan(raw: &super::tconstruct_modifier_snapshot::Raw, stack: &ItemStack,
    traits: &super::tconstruct_source_material_traits::Traits) -> Option<Plan> {
    let active = super::tconstruct_source_material_traits::level_with_upgrade(stack, traits, "tconstruct:overslime")? > 0;
    let amount = stored(stack, active)?;
    let bypass = if active {
        super::tconstruct_tanned_wear::member(raw, "tconstruct:durability/bypass_overslime", "tconstruct:empty")?
    } else { false };
    Some(Plan { amount, bypass })
}

/// Returns the remaining durability loss and shield to commit. Cost is exactly
/// one in the static Overslime modifier; even later Unbreakable cannot refund it.
pub(super) fn absorb(amount: u32, plan: Plan) -> (u32, u32) {
    let used = if plan.bypass { 0 } else { amount.min(plan.amount) };
    (amount - used, used)
}

#[cfg(test)] mod tests {
    use super::*;
    fn stack(amount: Tag, capacity: Tag) -> ItemStack {
        let mut stack = ItemStack::new("tconstruct:plate_helmet", 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tic_overslime", amount), ("tic_overslime_capacity", capacity),
        ])));
        stack
    }
    #[test] fn partial_and_complete_absorption_are_saved_separately() {
        assert_eq!(absorb(7, Plan { amount: 3, bypass: false }), (4, 3));
        assert_eq!(absorb(2, Plan { amount: 3, bypass: false }), (0, 2));
        assert_eq!(absorb(0, Plan { amount: 3, bypass: false }), (0, 0));
        assert_eq!(absorb(7, Plan { amount: 3, bypass: true }), (7, 0));
    }
    #[test] fn orphan_overcapacity_and_malformed_shields_are_rejected() {
        assert_eq!(stored(&stack(Tag::Int(3), Tag::Int(5)), true), Some(3));
        assert_eq!(stored(&stack(Tag::Int(3), Tag::Int(5)), false), None);
        assert_eq!(stored(&stack(Tag::Int(6), Tag::Int(5)), true), None);
        assert_eq!(stored(&stack(Tag::Int(-1), Tag::Int(5)), true), None);
        assert_eq!(stored(&stack(Tag::Long(3), Tag::Int(5)), true), None);
    }
    #[test] fn spent_shield_is_independent_of_final_durability_loss() {
        let mut item = stack(Tag::Int(3), Tag::Int(5));
        let (_, spent) = absorb(2, Plan { amount: 3, bypass: false });
        // A later Unbreakable/native cancellation can return zero durability
        // loss without undoing the earlier shield consumption.
        super::super::tconstruct_source_overslime::spend(&mut item, spent);
        assert_eq!(stored(&item, true), Some(1));
    }
    #[test] fn paired_commit_requires_one_final_loss_and_one_shield_record() {
        let item = stack(Tag::Int(3), Tag::Int(5));
        let slots = [Some(item.clone()), None, None, None];
        let losses = [(item.clone(), 0)];
        assert!(super::super::tconstruct_source_armor_wear::paired_losses(&slots, &losses, &[]).is_empty());
        assert_eq!(super::super::tconstruct_source_armor_wear::paired_losses(&slots, &losses, &[(item.clone(), 2)]), vec![(0, 0, 2)]);
        assert!(super::super::tconstruct_source_armor_wear::paired_losses(&slots, &losses, &[(item.clone(), 2), (item, 1)]).is_empty());
    }

}
