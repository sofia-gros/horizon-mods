//! Connects the damage-reducing modifiers to PlutoLib's item-wear event: a Tinkers tool a player wears down loses less
//! (or none) when its modifiers cancel points, as ToolDamageUtil.damage's modifier loop does in the original.
//! The tool's wear runs through ToolDamageUtil.damage over the stack (overslime, reductions, the tool's own damage and broken
//! state, the published durability, mining and attribute components) and the finished stack replaces the worn one; a piece the
//! event cannot rewrite (worn armor) keeps the host's own wear, less what the modifiers cancelled.
use crate::{break_speed_hook::with_held_tool, tool_slot, tool_stack};
use pluto_lib::ServerLivingEntityEvents;
use std::sync::atomic::{AtomicU64, Ordering};

static SEED: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

/// A uniform value in [0, 1) from a xorshift generator; modifier chances are drawn one point at a time.
pub(crate) fn random() -> f32 {
    let mut state = SEED.load(Ordering::Relaxed);
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    SEED.store(state, Ordering::Relaxed);
    (state >> 40) as f32 / (1u64 << 24) as f32
}

/// The wear after the tool's modifiers (the overslime shield soaks first, then each modifier cancels part of what is left, as
/// ToolDamageUtil.damage does) and the worn stack with that wear applied; the amount unchanged and no stack when the stack is not
/// a loaded Tinkers tool.
pub(crate) fn worn(stack: &pluto_lib::StackData, slot: Option<&pluto_lib::ItemSlot>, amount: i32) -> (i32, Option<pluto_lib::ItemSlot>) {
    let Ok(result) = with_held_tool(stack, |env, tool| {
        let mut data = tool.custom_data.clone();
        let soaked = crate::capacity::absorb(&mut data, env, None, amount, &mut random);
        let left = tool_stack::reduce_damage(&data, env, None, soaked, &mut random);
        let replacement = slot.and_then(|slot| {
            let mut next = slot.clone();
            // The same modifier loop and the tool's own damage and broken state, then the visible stack follows.
            tool_slot::damage_slot(&mut next, env, amount, true, true, None, &mut random).ok().map(|_| next)
        });
        (left, replacement)
    }) else { return (amount, None); };
    result
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::MODIFY_ITEM_DAMAGE.register(|wear| {
        static LOGGED: AtomicU64 = AtomicU64::new(0);
        let log = LOGGED.fetch_add(1, Ordering::Relaxed) < 6;
        let Some(stack) = wear.stack.as_ref() else {
            if log { eprintln!("Tinkers: item wear of {} by {}: the player's stack was not found", wear.item, wear.amount.get()); }
            return;
        };
        let amount = wear.amount.get();
        if amount <= 0 { return; }
        // The formulas of tool damage modules read the player (light, temperature) as the tool wore.
        let (left, replacement) = crate::entity_facts::with_holder(wear.holder.as_ref(), || worn(stack, wear.slot.as_ref(), amount));
        if left != amount { wear.amount.set(left); }
        if replacement.is_some() { *wear.replacement.borrow_mut() = replacement; }
    });
}

#[cfg(test)]
mod tests {
    use super::random;

    #[test]
    fn the_generator_stays_inside_zero_to_one() {
        for _ in 0..1000 { let value = random(); assert!((0.0..1.0).contains(&value), "{value}"); }
    }
}
