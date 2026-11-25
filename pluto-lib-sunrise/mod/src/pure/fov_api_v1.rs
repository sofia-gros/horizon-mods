//! The field of view a mod reshapes (the original's
//! `ModifierClientEvents.handleZoom` over the FOV modifier data): the
//! game computes a field-of-view modifier from the local player's state
//! each tick, and a callback answers the factor its own modifiers
//! contribute, which the transport multiplies in.
//!
//! In the original the contributions live in a per-player multiplier
//! keyed by modifier and slot, set as equipment changes and read by the
//! FOV event as one product. Here the product is recomputed from the
//! player's current equipment stacks on every ask: a callback reads
//! the stacks (held and worn alike), works out its factor from them,
//! and answers it, or answers None when nothing it owns contributes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

use super::block_entity_api_v1::ItemSlot;

/// The facts of one field-of-view computation, as the game states them
/// when it asks: the local player's state and equipment, and the
/// modifier the game computed from the state before any callback.
pub struct FovFacts<'a> {
    /// The field-of-view modifier the game computed from the player's
    /// state (flying, speed, the item in use), before callbacks: the
    /// value a callback's factor multiplies.
    pub base: f32,
    /// Whether the player is flying (the game widens the view for it in
    /// the base already; a callback reads it as a fact, not a duty).
    pub flying: bool,
    /// The player's walking speed attribute, as the game reports it.
    pub walking_speed: f32,
    /// The player's movement speed attribute, as the game reports it.
    pub movement_speed: f64,
    /// The player's equipment stacks, hands and armor, in no promised
    /// order: each names its slot, and a slot the player has empty is
    /// absent rather than present with an empty stack.
    pub stacks: &'a [FovStack<'a>],
    /// How many ticks the player has been using the item in use, or
    /// None when the player is not using anything: a zoom reads the
    /// count against its draw time.
    pub using_ticks: Option<i32>,
    /// Whether the worn-helmet keybind is held: a zoom module answers it with its keybind
    /// factor (the original's `startInteract` zoom), as the armor interact hook's key tracking
    /// reports it. A callback that does not answer the keybind ignores it.
    pub keybind_zoom: bool,
}

/// One equipment slot of the local player, as a [`FovFacts`] stack
/// names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FovSlot {
    Feet,
    Legs,
    Chest,
    Head,
    MainHand,
    OffHand,
}

/// One of the local player's equipment stacks, with the slot it sits
/// in.
#[derive(Clone, Copy, Debug)]
pub struct FovStack<'a> {
    /// The slot the stack sits in.
    pub slot: FovSlot,
    /// The stack itself, with its custom data (a tool's modifiers and
    /// persistent data ride there).
    pub stack: &'a ItemSlot,
}

/// Answers the factor one mod's modifiers contribute to the field of
/// view, or None when they contribute nothing this tick.
pub type FovCallback = fn(&FovFacts<'_>) -> Option<f32>;

static CALLBACKS: RwLock<Vec<FovCallback>> = RwLock::new(Vec::new());

/// Whether the worn-helmet keybind is held, as the armor interact hook last reported it: set on
/// the helmet key's press, cleared on its release. The transport reads it into the facts, so a
/// zoom module's keybind zoom needs no packet round trip to show.
static KEYBIND_ZOOM: AtomicBool = AtomicBool::new(false);

/// The registry of field-of-view callbacks. Register once during
/// startup; the transport asks every registered callback on each
/// field-of-view computation and multiplies the game's modifier by
/// the product of the factors they answer.
pub struct FovRegistry;

impl FovRegistry {
    /// Registering the same callback again does not register it twice.
    pub fn register(callback: FovCallback) {
        if let Ok(mut callbacks) = CALLBACKS.write() {
            if !callbacks.contains(&callback) {
                callbacks.push(callback);
            }
        }
    }

    /// The helmet key is held (`true`) or released (`false`): the keybind zoom's half of the
    /// state, written by the armor interact hook's key tracking.
    pub fn set_keybind_zoom(held: bool) {
        KEYBIND_ZOOM.store(held, Ordering::Relaxed);
    }
}

/// Whether the helmet key is held now, as the armor interact hook last reported it: the
/// transport reads it into the facts on each field-of-view computation.
pub fn keybind_zoom() -> bool {
    KEYBIND_ZOOM.load(Ordering::Relaxed)
}

/// The product of every registered callback's factor over `facts`, or
/// None when no callback contributes: the transport multiplies the
/// game's own modifier by this, and leaves it alone when None.
pub fn fov_multiplier(facts: &FovFacts<'_>) -> Option<f32> {
    let callbacks = CALLBACKS.read().ok()?;
    let mut product: Option<f32> = None;
    for callback in callbacks.iter() {
        if let Some(factor) = callback(facts) {
            product = Some(product.map_or(factor, |total| total * factor));
        }
    }
    product
}

#[cfg(test)]
mod tests {
    use super::super::base::Identifier;
    use super::*;

    fn facts<'a>(stacks: &'a [FovStack<'a>]) -> FovFacts<'a> {
        FovFacts { base: 1.0, flying: false, walking_speed: 0.1, movement_speed: 0.1, stacks, using_ticks: None, keybind_zoom: false }
    }

    #[test]
    fn contributions_multiply_and_silence_leaves_the_view_alone() {
        assert_eq!(fov_multiplier(&facts(&[])), None, "no callbacks registered yet");
        FovRegistry::register(|_| None);
        assert_eq!(fov_multiplier(&facts(&[])), None, "a callback that never speaks changes nothing");
        FovRegistry::register(|_| Some(1.05));
        FovRegistry::register(|facts| facts.stacks.first().map(|_| 0.5));
        let stack = ItemSlot::new(Identifier::new("test", "helm").unwrap(), 1);
        let stacks = [FovStack { slot: FovSlot::Head, stack: &stack }];
        assert_eq!(fov_multiplier(&facts(&stacks)), Some(1.05 * 0.5));
        assert_eq!(fov_multiplier(&facts(&[])), Some(1.05));
    }

    #[test]
    fn the_keybind_zoom_flag_is_set_on_press_and_cleared_on_release() {
        assert!(!keybind_zoom(), "nothing held it yet");
        FovRegistry::set_keybind_zoom(true);
        assert!(keybind_zoom());
        FovRegistry::set_keybind_zoom(false);
        assert!(!keybind_zoom());
    }
}
