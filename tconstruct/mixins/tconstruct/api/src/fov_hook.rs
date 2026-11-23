//! The field-of-view modifiers (the original's `FovModule` and
//! `ZoomModule`, SlimeKnights, MIT; see LICENSE) over PlutoLib's FOV
//! surface: the original sets a per-modifier multiplier as equipment
//! changes and multiplies the FOV event by their product; the port's
//! callback recomputes the product from the player's current stacks on
//! every ask.
//!
//! Documented differences from the original: a tool that breaks while
//! worn stops contributing here (the original's multiplier goes only
//! when the piece comes off, and its unequip skips a broken piece, so
//! there the factor can outlive the break); the hand using the item is
//! inferred main hand first, as the FOV facts do not name it; and the
//! worn-helmet keybind zoom (the armor interact hook) and the spyglass
//! use and stop sounds are not wired — the held tool's zoom while it
//! is used is.

use pluto_lib::fov_api_v1::{FovFacts, FovRegistry, FovSlot};
use pluto_lib::{ItemSlot, Nbt};

use crate::modifier_modules::{Module, ZoomStyle};
use crate::{modifier_entries, tool_stack};

/// `ZoomModule.KEY_DRAWTIME`: the persistent int a scope's draw time is
/// stored under.
const KEY_DRAWTIME: &str = "tconstruct:drawtime";
/// ZoomModule's spyglass factor, set on the first using tick.
const SPYGLASS_FACTOR: f32 = 0.1;
/// ZoomModule's scope zoom: the factor falls by this share of the view
/// over the draw.
const SCOPE_ZOOM: f32 = 0.6;
/// ZoomModule's draw time when the stored one is not positive.
const DEFAULT_DRAW_TIME: i32 = 20;

fn multiplier(facts: &FovFacts<'_>) -> Option<f32> {
    let guard = crate::common::server_modifiers()?.read().ok()?;
    let registry: &crate::modifiers::Registry = &guard;
    let mut product: Option<f32> = None;
    let mut multiply = |factor: f32| product = Some(product.map_or(factor, |total| total * factor));
    // FovModule: every equipped stack's entries contribute, held and
    // worn alike, as the equipment change hook set them in the original.
    for fov_stack in facts.stacks {
        let stack = fov_stack.stack;
        if tool_stack::is_broken(&stack.custom_data) { continue; }
        for (entry, modifier) in tool_stack::saved_modifiers(&stack.custom_data, registry) {
            for module in modifier.modules() {
                if let Some(factor) = module.fov_factor(&entry) { multiply(factor); }
            }
        }
    }
    // ZoomModule: the tool in use narrows the view, the main hand first.
    if let Some(ticks) = facts.using_ticks {
        for slot in [FovSlot::MainHand, FovSlot::OffHand] {
            let Some(fov_stack) = facts.stacks.iter().find(|stack| stack.slot == slot) else { continue; };
            let stack = fov_stack.stack;
            if tool_stack::is_broken(&stack.custom_data) { continue; }
            let style = tool_stack::saved_modifiers(&stack.custom_data, registry).iter()
                .find_map(|(_, modifier)| modifier.modules().iter().find_map(Module::zoom_style));
            let Some(style) = style else { continue; };
            match style {
                ZoomStyle::Spyglass => multiply(SPYGLASS_FACTOR),
                ZoomStyle::Scope if ticks > 0 => {
                    let draw_time = draw_time(stack);
                    multiply(1.0 - SCOPE_ZOOM * (ticks as f32 / draw_time as f32).min(1.0));
                }
                ZoomStyle::Scope => {}
            }
            break;
        }
    }
    product
}

/// A scope's draw time: the tool's persistent `tconstruct:drawtime`,
/// or the module's fallback when it is not positive.
fn draw_time(stack: &ItemSlot) -> i32 {
    let stored = match stack.custom_data.get(tool_stack::TAG_PERSISTENT) {
        Some(Nbt::Compound(persistent)) => modifier_entries::nbt_int(persistent.get(KEY_DRAWTIME)),
        _ => 0,
    };
    if stored > 0 { stored } else { DEFAULT_DRAW_TIME }
}

/// Installs the field-of-view callback.
pub(crate) fn register() {
    FovRegistry::register(multiplier);
}
