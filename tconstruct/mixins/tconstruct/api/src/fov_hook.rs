//! The field-of-view modifiers (the original's `FovModule` and
//! `ZoomModule`, SlimeKnights, MIT; see LICENSE) over PlutoLib's FOV
//! surface: the original sets a per-modifier multiplier as equipment
//! changes and multiplies the FOV event by their product; the port's
//! callback recomputes the product from the player's current stacks on
//! every ask.
//!
//! The worn-helmet keybind zoom is wired through the armor interact hook: the hook's key tracking
//! sets the keybind flag on the helmet key's press and clears it on its release, and the callback
//! answers the spyglass factor while the flag is held and the head piece carries a spyglass-style
//! zoom module (the scope binds no armor interact hook, so it answers the keybind with nothing).
//! The spyglass sounds ride the use: the first using tick of a spyglass tool plays its use sound,
//! and letting go of a zoom tool plays its stop sound; the keybind's own use and stop sounds are
//! played by the armor interact hook's receiver, through the player tick (see there).
//!
//! Documented differences from the original: a tool that breaks while
//! worn stops contributing here (the original's multiplier goes only
//! when the piece comes off, and its unequip skips a broken piece, so
//! there the factor can outlive the break); the hand using the item is
//! inferred main hand first, as the FOV facts do not name it.

use pluto_lib::fov_api_v1::{FovFacts, FovRegistry, FovSlot, FovStack};
use pluto_lib::{ItemEvents, ItemSlot, Nbt, NbtCompound, ReleaseUsing, StackData, UseTick};

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

/// The zoom style a stack's saved modifiers give it, if any: a broken tool's modules do not run.
fn zoom_style(data: &NbtCompound, registry: &crate::modifiers::Registry) -> Option<ZoomStyle> {
    if tool_stack::is_broken(data) { return None; }
    tool_stack::saved_modifiers(data, registry).iter()
        .find_map(|(_, modifier)| modifier.modules().iter().find_map(Module::zoom_style))
}

/// The zoom style a stack carries, over the loaded modifier registry: None when the registry is
/// not loaded, the stack is not a Tinkers tool, or no modifier gives it a zoom style.
pub(crate) fn zoom_style_of(stack: &StackData) -> Option<ZoomStyle> {
    let guard = crate::common::server_modifiers()?.read().ok()?;
    let registry: &crate::modifiers::Registry = &guard;
    zoom_style(&stack.custom_data, registry)
}

/// ZoomModule.startInteract's factor: the spyglass factor while the helmet key is held and the
/// head piece carries a spyglass-style zoom module. The scope binds no armor interact hook, so
/// a scope helmet answers the keybind with nothing.
fn keybind_factor(facts: &FovFacts<'_>, registry: &crate::modifiers::Registry) -> Option<f32> {
    if !facts.keybind_zoom { return None; }
    let head = facts.stacks.iter().find(|stack| stack.slot == FovSlot::Head)?;
    if zoom_style(&head.stack.custom_data, registry) == Some(ZoomStyle::Spyglass) {
        Some(SPYGLASS_FACTOR)
    } else {
        None
    }
}

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
    // ZoomModule.startInteract: the worn-helmet keybind zoom, the spyglass style only.
    if let Some(factor) = keybind_factor(facts, registry) { multiply(factor); }
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

/// ZoomModule.onUsingTick: the spyglass plays its use sound on the first using tick (the scope's
/// tick is its progressive zoom, already in the factor, and plays none).
fn on_using_tick(tick: &UseTick) {
    // The first ticks of a use read near 0; 1 is the safe first tick in either count (it plays
    // once, at the start, whether the count starts at 0 or 1).
    if tick.ticks_used != 1 { return; }
    if zoom_style_of(&tick.stack) != Some(ZoomStyle::Spyglass) { return; }
    tick.world().play_sound("minecraft:item.spyglass.use", tick.position, 1.0, 1.0);
}

/// ZoomModule.afterStopUsing: letting go of a zoom tool plays its stop sound (either style: the
/// use's end is the module's shared cleanup).
fn on_release(release: &ReleaseUsing) {
    if zoom_style_of(&release.stack).is_none() { return; }
    release.world().play_sound("minecraft:item.spyglass.stop_using", release.position, 1.0, 1.0);
}

/// Installs the field-of-view callback and the zoom use sounds.
pub(crate) fn register() {
    FovRegistry::register(multiplier);
    ItemEvents::USE_TICK.register(on_using_tick);
    ItemEvents::RELEASE_USING.register(on_release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modifier_entries::Entry;
    use crate::modifiers::{Modifier, Registry};
    use pluto_lib::Identifier;
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    /// A registry holding one zoom modifier of `style`, for the style lookups.
    fn registry(style: &str) -> Registry {
        let mut registry = Registry::default();
        registry.register_static(Modifier::decode(id("tconstruct:zoom"), &json!({"modules": [{
            "type": "tconstruct:zoom",
            "style": style,
            "hooks": ["tconstruct:general_interact", "tconstruct:tool_using", "tconstruct:equipment_change", "tconstruct:armor_interact"],
        }]})).unwrap()).unwrap();
        registry
    }

    /// A stack carrying the zoom modifier at level 1 (the `tic_modifiers` list the saved
    /// modifiers read).
    fn zoom_stack() -> ItemSlot {
        let mut stack = ItemSlot::new(id("tconstruct:plate_helmet"), 1);
        let mut entry = NbtCompound::new();
        entry.insert("name".to_owned(), Nbt::String("tconstruct:zoom".to_owned()));
        entry.insert("level".to_owned(), Nbt::Int(1));
        stack.custom_data.insert(tool_stack::TAG_MODIFIERS.to_owned(), Nbt::List(vec![Nbt::Compound(entry)]));
        stack
    }

    fn facts<'a>(stacks: &'a [FovStack<'a>], keybind_zoom: bool) -> FovFacts<'a> {
        FovFacts { base: 1.0, flying: false, walking_speed: 0.1, movement_speed: 0.1, stacks, using_ticks: None, keybind_zoom }
    }

    #[test]
    fn the_zoom_style_is_read_from_the_saved_modifiers_and_a_broken_tool_has_none() {
        let spyglass = registry("spyglass");
        let scope = registry("scope");
        let stack = zoom_stack();
        assert_eq!(zoom_style(&stack.custom_data, &spyglass), Some(ZoomStyle::Spyglass));
        assert_eq!(zoom_style(&stack.custom_data, &scope), Some(ZoomStyle::Scope));
        assert_eq!(zoom_style(&ItemSlot::new(id("tconstruct:plate_helmet"), 1).custom_data, &spyglass), None, "no zoom modifier");
        let mut broken = zoom_stack();
        tool_stack::set_damage(&mut broken.custom_data, 99999);
        assert!(tool_stack::is_broken(&broken.custom_data), "damage past durability breaks it");
        assert_eq!(zoom_style(&broken.custom_data, &spyglass), None, "a broken tool's modules do not run");
    }

    #[test]
    fn the_keybind_factor_is_the_spyglass_factor_while_the_key_is_held() {
        let spyglass = registry("spyglass");
        let stack = zoom_stack();
        let stacks = [FovStack { slot: FovSlot::Head, stack: &stack }];
        assert_eq!(keybind_factor(&facts(&stacks, true), &spyglass), Some(SPYGLASS_FACTOR));
        assert_eq!(keybind_factor(&facts(&stacks, false), &spyglass), None, "the key released answers nothing");
        let scope = registry("scope");
        assert_eq!(keybind_factor(&facts(&stacks, true), &scope), None, "a scope helmet answers the keybind with nothing");
        assert_eq!(keybind_factor(&facts(&[], true), &spyglass), None, "no helmet, no keybind zoom");
    }

    #[test]
    fn a_scope_draw_time_comes_from_the_persistent_data_or_the_fallback() {
        let mut stack = ItemSlot::new(id("tconstruct:longbow"), 1);
        assert_eq!(draw_time(&stack), DEFAULT_DRAW_TIME, "nothing stored");
        let mut persistent = NbtCompound::new();
        persistent.insert(KEY_DRAWTIME.to_owned(), Nbt::Int(40));
        stack.custom_data.insert(tool_stack::TAG_PERSISTENT.to_owned(), Nbt::Compound(persistent));
        assert_eq!(draw_time(&stack), 40);
    }
}
