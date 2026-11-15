//! ToolDamageUtil damage/repair rules from SlimeKnights (MIT; see LICENSE).
//! Pure NBT arithmetic. Modifier TOOL_DAMAGE/DURABILITY_CHANGED hooks, criteria triggers
//! and break animations belong to the caller; none of those hooks exist in this port yet,
//! so the returned amounts are what such hooks would observe.
use crate::tool_stack::{damage, durability, is_broken, set_damage, TAG_BROKEN};
use pluto_lib::{Nbt, NbtCompound};

const TAG_UNBREAKABLE: &str = "Unbreakable";

/// ToolDamageUtil.breakTool: raw flag only, damage is left to the next setDamage.
pub(crate) fn break_tool(nbt: &mut NbtCompound) { nbt.insert(TAG_BROKEN.into(), Nbt::Byte(1)); }

pub(crate) fn is_unbreakable(nbt: &NbtCompound) -> bool {
    nbt.get(TAG_UNBREAKABLE).and_then(Nbt::as_i64).is_some_and(|value| value != 0)
}

/// Item.getMaxDamage replacement: one higher while broken so vanilla never deletes the stack.
pub(crate) fn fake_max_damage(nbt: &NbtCompound, can_be_depleted: bool) -> i32 {
    if !can_be_depleted { return 0; }
    let durability = durability(nbt);
    if is_broken(nbt) { durability.wrapping_add(1) } else { durability }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Damaged {
    /// Damage actually applied after the remaining-durability cap; modifier hooks see this.
    pub applied: i32,
    pub broke: bool,
}

/// directDamage: bypasses modifier hooks. `consumes_resources` is false for creative players.
pub(crate) fn direct_damage(nbt: &mut NbtCompound, amount: i32, consumes_resources: bool) -> Damaged {
    if !consumes_resources { return Damaged { applied: 0, broke: false }; }
    let durability = durability(nbt);
    let current_damage = damage(nbt);
    let amount = amount.min(durability.wrapping_sub(current_damage));
    if amount <= 0 { return Damaged { applied: 0, broke: false }; }
    let new_damage = current_damage.wrapping_add(amount);
    set_damage(nbt, new_damage);
    Damaged { applied: amount, broke: new_damage >= durability }
}

/// damage: the original early-outs, then directDamage. `has_durability_tag` is the item's
/// tconstruct:durability tag membership, which only the caller can know.
pub(crate) fn damage_tool(nbt: &mut NbtCompound, amount: i32, consumes_resources: bool, has_durability_tag: bool) -> Damaged {
    if amount <= 0 || is_broken(nbt) || is_unbreakable(nbt) || !has_durability_tag {
        return Damaged { applied: 0, broke: false };
    }
    direct_damage(nbt, amount, consumes_resources)
}

/// repair: never repairs past full health; returns the amount repaired for after-repair hooks.
pub(crate) fn repair(nbt: &mut NbtCompound, amount: i32) -> i32 {
    if amount <= 0 { return 0; }
    let current = damage(nbt);
    if current == 0 { return 0; }
    let amount = amount.min(current);
    set_damage(nbt, current - amount);
    amount
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_stack::{current_durability, damage_raw, TAG_STATS};
    fn tool(durability: f32) -> NbtCompound {
        let mut stats = NbtCompound::new();
        stats.insert("tconstruct:durability".into(), Nbt::Float(durability));
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_STATS.into(), Nbt::Compound(stats));
        nbt
    }
    #[test]
    fn damage_caps_at_remaining_durability_and_breaks_exactly_at_zero() {
        let mut nbt = tool(10.0);
        assert_eq!(damage_tool(&mut nbt, 4, true, true), Damaged { applied: 4, broke: false });
        assert_eq!((damage(&nbt), current_durability(&nbt)), (4, 6));
        // Overkill applies only what remains and reports the break.
        assert_eq!(damage_tool(&mut nbt, 100, true, true), Damaged { applied: 6, broke: true });
        assert!(is_broken(&nbt));
        assert_eq!((damage_raw(&nbt), damage(&nbt), current_durability(&nbt)), (10, 10, 0));
        // A broken tool takes no further damage.
        assert_eq!(damage_tool(&mut nbt, 1, true, true), Damaged { applied: 0, broke: false });
        assert_eq!(fake_max_damage(&nbt, true), 11, "vanilla must never see max == damage");
        assert_eq!(fake_max_damage(&nbt, false), 0);
    }
    #[test]
    fn creative_unbreakable_untagged_and_nonpositive_damage_do_nothing() {
        let mut nbt = tool(10.0);
        assert_eq!(damage_tool(&mut nbt, 3, false, true).applied, 0);
        assert_eq!(damage_tool(&mut nbt, 3, true, false).applied, 0);
        assert_eq!(damage_tool(&mut nbt, 0, true, true).applied, 0);
        assert_eq!(damage_tool(&mut nbt, -5, true, true).applied, 0);
        nbt.insert(TAG_UNBREAKABLE.into(), Nbt::Byte(1));
        assert_eq!(damage_tool(&mut nbt, 3, true, true).applied, 0);
        assert_eq!(damage_raw(&nbt), 0);
        // directDamage bypasses the unbreakable/tag early-outs but not resource consumption.
        assert_eq!(direct_damage(&mut nbt, 3, true).applied, 3);
        assert_eq!(direct_damage(&mut nbt, 3, false).applied, 0);
    }
    #[test]
    fn repair_never_exceeds_full_and_revives_a_broken_tool() {
        let mut nbt = tool(10.0);
        assert_eq!(repair(&mut nbt, 5), 0, "undamaged tools are untouched");
        damage_tool(&mut nbt, 100, true, true);
        assert!(is_broken(&nbt));
        assert_eq!(repair(&mut nbt, 4), 4);
        assert!(!is_broken(&nbt));
        assert_eq!((damage(&nbt), current_durability(&nbt)), (6, 4));
        assert_eq!(repair(&mut nbt, 100), 6);
        assert_eq!((damage(&nbt), current_durability(&nbt)), (0, 10));
        assert_eq!(repair(&mut nbt, 0), 0);
        assert_eq!(repair(&mut nbt, -3), 0);
    }
    #[test]
    fn break_tool_sets_only_the_flag() {
        let mut nbt = tool(10.0);
        break_tool(&mut nbt);
        assert!(is_broken(&nbt));
        assert_eq!(damage_raw(&nbt), 0);
        assert_eq!(current_durability(&nbt), 0);
    }
}
