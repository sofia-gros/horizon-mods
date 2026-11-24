//! The experience multiplier at work (ModifierEvents in the original): the experience a
//! block break drops and the experience a mob drops on death are scaled by the responsible
//! player's `tconstruct:player.experience_multiplier` attribute, with the terms the original
//! adds at each site. PlutoLib hears the two moments as its experience-drop event (a mob
//! killed by a player, before the orbs split) and its block-break experience event (the
//! player's own break, as the drop is sampled); this module answers both.
//!
//! Block break (ModifierEvents.beforeBlockBreak): the bonus is the player's attribute value,
//! plus half the held tool's `experienced` level, plus the player's armor statistic for
//! experience — a statistic no shipped content writes (it is an extension point for addons),
//! so that term is 0 here as it is for the shipped original. The dropped experience is the
//! sampled amount times the bonus, truncated toward zero as the original's `(int)` cast does.
//!
//! Mob death (ModifierEvents.onExperienceDrop): the multiplier starts at 1, plus the dying
//! mob's `experienced` effect level, plus for a player killer their attribute value and the
//! same (unwritten) armor statistic. The last term is the killing projectile's recorded
//! experience float when the mob carries one — worked out from the stack it was launched from,
//! remembered at its launch — and otherwise the volatile `tconstruct:experience` float of the
//! tool in the looting slot (the `experienced` modifier's per-level 0.5), usually the main
//! hand, an armor piece or the off hand when the killing blow claimed one. SlimeKnights (MIT;
//! see LICENSE).
use crate::{break_speed_hook::with_held_tool, custom_attributes, tool_stack};
use pluto_lib::{ExperienceDrop, Nbt, NbtCompound, PlayerBlockBreakEvents, ServerLivingEntityEvents, ServerPlayer, StackData, StackSlot};

/// The `experienced` modifier, whose level feeds the block-break bonus.
const EXPERIENCED: &str = "tconstruct:experienced";
/// The volatile float key the `experienced` modifier records on a tool (and on a projectile
/// it launches): experience per level, 0.5.
const EXPERIENCE_FLAG: &str = "tconstruct:experience";
/// The `experienced` effect, whose level on the dying mob feeds the death multiplier.
const EXPERIENCED_EFFECT: &str = "tconstruct:experienced";

pub(crate) fn register() {
    ServerLivingEntityEvents::EXPERIENCE_DROP.register(|drop| {
        let amplifier = drop.effects.iter().find(|(id, _)| id.as_str() == EXPERIENCED_EFFECT).map(|(_, amplifier)| *amplifier);
        let player = drop.player.map(|(uuid, entity)| {
            let attribute = custom_attributes::value(entity, custom_attributes::EXPERIENCE_MULTIPLIER);
            (attribute, looting_tool_experience(drop, uuid))
        });
        let multiplier = mob_multiplier(amplifier, player, projectile_boost(drop));
        drop.experience.set(scaled(drop.experience.get(), multiplier));
    });
    PlayerBlockBreakEvents::EXPERIENCE.register(|context| {
        if context.player.is_none() { return; }
        let attribute = custom_attributes::value(context.player_entity, custom_attributes::EXPERIENCE_MULTIPLIER);
        let level = context.held.as_ref()
            .and_then(|held| with_held_tool(held, |env, stack| tool_stack::modifier_level(&stack.custom_data, env, EXPERIENCED)).ok())
            .unwrap_or(0);
        let bonus = block_bonus(attribute, level);
        context.experience.set((f64::from(context.experience.get()) * bonus) as u32);
    });
}

/// The volatile experience float recorded on `stack` (the `experienced` modifier's per-level
/// amount); 0 when it is no Tinkers tool or records none.
fn volatile_experience(stack: &StackData) -> f64 {
    with_held_tool(stack, |_env, stack| match stack.custom_data.get(tool_stack::TAG_VOLATILE) {
        Some(Nbt::Compound(volatile)) => volatile.get(EXPERIENCE_FLAG).and_then(Nbt::as_f64).unwrap_or(0.0),
        _ => 0.0,
    }).unwrap_or(0.0)
}

/// The held tool's volatile experience float for the player `uuid`; 0 when they hold no Tinkers
/// tool or it records none.
fn held_tool_experience(uuid: u128) -> f64 {
    let player = ServerPlayer { uuid, name: String::new() };
    player.main_hand_stack().map(|held| volatile_experience(&held)).unwrap_or(0.0)
}

/// The killing projectile's recorded experience float, worked out from the stack it was launched
/// from (`tool_stack::launch_volatile_floats` over the launcher's tool, as the original's
/// `onLivingDeath` copies the projectile's persistent float onto the dying mob): `Some` only when
/// the launcher is a Tinkers tool, so a non-tool launcher falls back to the held tool; a computed
/// 0.0 still counts as `Some(0.0)`, as the original's recorded float replaces the held tool even
/// when it is zero. None when no projectile killed the mob.
fn projectile_boost(drop: &ExperienceDrop) -> Option<f64> {
    let tool = drop.projectile_tool.as_ref()?;
    with_held_tool(tool, |env, stack| {
        let mut floats = NbtCompound::new();
        tool_stack::launch_volatile_floats(&stack.custom_data, env, &mut floats);
        floats.get(EXPERIENCE_FLAG).and_then(Nbt::as_f64)
    }).ok().flatten()
}

/// The slot number `slot` reads as on the player, for the looting slot's live read; None for the
/// main hand, which is the selected hotbar slot and has no fixed number (the host's inventory
/// numbers: 100 feet to 103 head for armor, -106 for the off hand).
fn looting_slot_number(slot: StackSlot) -> Option<i32> {
    match slot {
        StackSlot::MainHand => None,
        StackSlot::OffHand => Some(-106),
        StackSlot::Feet => Some(100),
        StackSlot::Legs => Some(101),
        StackSlot::Chest => Some(102),
        StackSlot::Head => Some(103),
    }
}

/// The volatile experience float of the tool in the looting slot the killing blow claimed
/// (canon's `ModifierLootingHandler.getLootingSlot`): the armor piece or off-hand stack the blow
/// was struck with, read live from its slot; the main hand when no blow named one, and 0 when the
/// claimed slot holds no Tinkers tool (the original reads no float then, not the main hand's).
fn looting_tool_experience(drop: &ExperienceDrop, uuid: u128) -> f64 {
    let Some(slot) = drop.looting_slot else { return held_tool_experience(uuid) };
    let Some(number) = looting_slot_number(slot) else { return held_tool_experience(uuid) };
    let player = ServerPlayer { uuid, name: String::new() };
    player.stack(number).map(|stack| volatile_experience(&stack)).unwrap_or(0.0)
}

/// The mob death multiplier (ModifierEvents.onExperienceDrop): 1, plus the dying mob's
/// `experienced` effect level (`effect_amplifier`, absent for none), plus for a player killer
/// (`player`: their attribute value and their held tool's volatile float) the attribute — the
/// armor statistic term is 0, as no shipped content writes it — plus the killing projectile's
/// recorded float (`projectile_boost`) when the mob carries one, and otherwise the held
/// tool's float.
pub(crate) fn mob_multiplier(effect_amplifier: Option<i32>, player: Option<(f64, f64)>, projectile_boost: Option<f64>) -> f64 {
    let mut multiplier = 1.0 + f64::from(effect_amplifier.unwrap_or(0));
    if let Some((attribute, _)) = player { multiplier += attribute; }
    match projectile_boost {
        Some(boost) => multiplier += boost,
        None => if let Some((_, tool_float)) = player { multiplier += tool_float; },
    }
    multiplier
}

/// The block break bonus (ModifierEvents.beforeBlockBreak): the player's attribute value
/// plus half their held tool's `experienced` level; the armor statistic term is 0, as above.
pub(crate) fn block_bonus(attribute: f64, experienced_level: i32) -> f64 {
    attribute + 0.5 * f64::from(experienced_level)
}

/// The original's `(int)(amount * factor)`: the product truncated toward zero.
pub(crate) fn scaled(amount: i32, factor: f64) -> i32 {
    (f64::from(amount) * factor) as i32
}

#[cfg(test)]
mod tests {
    use pluto_lib::StackSlot;
    use super::{block_bonus, looting_slot_number, mob_multiplier, scaled};

    #[test]
    fn mob_multiplier_stacks_its_terms() {
        // Nothing applies: the drop stands.
        assert_eq!(mob_multiplier(None, None, None), 1.0);
        // The dying mob's effect alone: 1 + its level.
        assert_eq!(mob_multiplier(Some(2), None, None), 3.0);
        // A player killer: their attribute joins (its base is 1 for an unmodified player).
        assert_eq!(mob_multiplier(None, Some((1.0, 0.0)), None), 2.0);
        // The held tool's volatile float (experienced 3 = 1.5) joins without a projectile.
        assert_eq!(mob_multiplier(Some(1), Some((1.25, 1.5)), None), 4.75);
        // A recorded projectile float replaces the held tool's, and lands without a player.
        assert_eq!(mob_multiplier(None, Some((1.0, 1.5)), Some(2.5)), 4.5);
        assert_eq!(mob_multiplier(None, None, Some(2.5)), 3.5);
        // A recorded projectile float of 0.0 still replaces the held tool's, as the original's
        // recorded float does even when it is zero.
        assert_eq!(mob_multiplier(None, Some((1.0, 1.5)), Some(0.0)), 2.0);
    }

    #[test]
    fn looting_slots_read_as_the_hosts_inventory_numbers() {
        assert_eq!(looting_slot_number(StackSlot::MainHand), None, "the main hand is the selected hotbar slot");
        assert_eq!(looting_slot_number(StackSlot::OffHand), Some(-106));
        assert_eq!([StackSlot::Feet, StackSlot::Legs, StackSlot::Chest, StackSlot::Head].map(|slot| looting_slot_number(slot)), [Some(100), Some(101), Some(102), Some(103)]);
    }

    #[test]
    fn block_bonus_is_attribute_plus_half_the_level() {
        assert_eq!(block_bonus(1.0, 0), 1.0);
        assert_eq!(block_bonus(1.0, 3), 2.5);
        assert_eq!(block_bonus(0.5, 2), 1.5);
    }

    #[test]
    fn scaled_truncates_toward_zero() {
        assert_eq!(scaled(7, 1.5), 10);
        assert_eq!(scaled(5, 0.5), 2);
        assert_eq!(scaled(0, 3.0), 0);
        assert_eq!(scaled(7, -1.5), -10);
        assert_eq!(scaled(10, 1.0), 10);
    }
}
