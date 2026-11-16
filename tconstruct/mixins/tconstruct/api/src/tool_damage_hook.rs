//! Connects the damage-reducing modifiers to PlutoLib's item-wear event: a Tinkers tool a player wears down loses less
//! (or none) when its modifiers cancel points, as ToolDamageUtil.damage's modifier loop does in the original.
//! The host applies the remaining wear itself; the tool's own damage NBT is not written here.
use crate::{break_speed_hook::with_held_tool, tool_stack};
use pluto_lib::ServerLivingEntityEvents;
use std::sync::atomic::{AtomicU64, Ordering};

static SEED: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

/// A uniform value in [0, 1) from a xorshift generator; modifier chances are drawn one point at a time.
fn random() -> f32 {
    let mut state = SEED.load(Ordering::Relaxed);
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    SEED.store(state, Ordering::Relaxed);
    (state >> 40) as f32 / (1u64 << 24) as f32
}

/// The wear left after the tool's modifiers; the amount unchanged when the stack is not a loaded Tinkers tool.
pub(crate) fn reduced(stack: &pluto_lib::StackData, amount: i32) -> i32 {
    with_held_tool(stack, |env, tool| tool_stack::reduce_damage(&tool.custom_data, env, None, amount, &mut random)).unwrap_or(amount)
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::MODIFY_ITEM_DAMAGE.register(|wear| {
        let Some(stack) = wear.stack.as_ref() else { return; };
        let amount = wear.amount.get();
        if amount <= 0 { return; }
        let left = reduced(stack, amount);
        if left != amount { wear.amount.set(left); }
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
