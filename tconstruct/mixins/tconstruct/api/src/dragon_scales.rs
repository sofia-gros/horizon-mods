//! Dragon scales, from ToolEvents.livingDamage in the original: when a player-caused explosion hurts the
//! ender dragon, the dragon drops dragon scales at itself, the count growing with the damage. The original
//! first cuts the amount to the health the dragon has left (a local clamp for its remaining handlers, the
//! event's amount itself is untouched), and only a hit of 2 or more then drops
//! `1 + random.nextInt(min(5, (int)(amount / 2)))` scales. PlutoLib's damage event reports whether the
//! causing entity is a player and the victim's health as the hit arrived, and the drop is an
//! [`EntityAction::DropItem`] on the victim, which the game spawns at the dragon.
use crate::{config, protection_hook, tool_damage_hook};
use pluto_lib::{ActionTarget, EntityAction, Identifier, ServerLivingEntityEvents, StackData};

/// The scale count for a hit of `amount` (already cut to the dragon's health), with `roll` a uniform value
/// in [0, 1): zero under the original's 2 damage floor, else `1 + nextInt(min(5, (int)(amount / 2)))`.
pub(crate) fn scale_count(amount: f32, roll: f32) -> u32 {
    if amount < 2.0 { return 0; }
    let bound = ((amount / 2.0) as i32).min(5);
    if bound <= 0 { return 1; }
    1 + (roll.clamp(0.0, 1.0 - f32::EPSILON) * bound as f32) as u32
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
        if hit.damage.kind != "minecraft:ender_dragon" { return; }
        if !config::drop_dragon_scales() { return; }
        // The causing entity must be a player (the dragon's own explosion sources name it, a crystal's
        // owner included); the game names it in the parties of the hit.
        if hit.attacker_is_player != Some(true) { return; }
        let Ok(damage_type) = Identifier::parse(&hit.damage.damage_type) else { return; };
        if !protection_hook::tags().contains_vanilla("is_explosion", &damage_type) { return; }
        let amount = hit.amount.get();
        // The original's local clamp: the count is worked out from what the dragon can still take.
        let clamped = hit.target_health.map_or(amount, |health| amount.min(health));
        let count = scale_count(clamped, tool_damage_hook::random());
        if count == 0 { return; }
        hit.actions.borrow_mut().push(EntityAction::DropItem {
            target: ActionTarget::Victim,
            stack: StackData { item: "tconstruct:dragon_scale".into(), count, ..StackData::default() },
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_follow_the_originals_count() {
        assert_eq!(scale_count(1.99, 0.0), 0, "under the 2 damage floor nothing drops");
        assert_eq!(scale_count(2.0, 0.0), 1, "nextInt(1) is always 0");
        assert_eq!(scale_count(2.0, 0.99), 1);
        assert_eq!(scale_count(3.9, 0.0), 1, "(int)(3.9 / 2) is 1");
        assert_eq!(scale_count(4.0, 0.0), 1, "a 0 roll on nextInt(2) adds nothing");
        assert_eq!(scale_count(4.0, 0.5), 2);
        assert_eq!(scale_count(100.0, 0.99), 5, "the count caps at 1 + nextInt(5)");
        assert_eq!(scale_count(100.0, 0.0), 1);
    }
}
