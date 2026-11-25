// Hanging variant transport: the game reads a frame or stand variant's
// facts once per interaction and asks the API crate for the registered
// variant's mechanics. The facts are plain data; the registry and the
// rotation/comparator/timer logic stay on the API side of the provider.
use std::sync::OnceLock;

/// One item frame variant's facts, as the game side needs them.
#[derive(Clone)]
pub struct FrameFacts {
    pub item: String,
    pub entity_type: String,
    pub variant: i32,
    pub rotation_step: i32,
    pub winds_down: bool,
    pub more_rotations: bool,
    pub fullbright: bool,
    pub hidden_when_filled: bool,
    pub click_through: bool,
    pub fire_immune: bool,
}

/// One armor stand variant's facts, as the game side needs them.
#[derive(Clone)]
pub struct StandFacts {
    pub item: String,
    pub entity_type: String,
    pub variant: i32,
    pub small: bool,
    pub arms: bool,
    pub fullbright: bool,
    pub hidden_when_equipped: bool,
}

pub struct Provider {
    pub frame_for_item: fn(&str) -> Option<FrameFacts>,
    pub frame_for_variant: fn(i32) -> Option<FrameFacts>,
    pub stand_for_item: fn(&str) -> Option<StandFacts>,
    pub stand_for_variant: fn(i32) -> Option<StandFacts>,
    /// `FrameVariant::set_rotation` for the variant the facts describe.
    pub frame_rotation: fn(&FrameFacts, i32) -> i32,
    /// `FrameVariant::analog_output` for the variant the facts describe.
    pub frame_analog: fn(&FrameFacts, bool, i32) -> i32,
    /// `FrameVariant::tick_rotation`: (new timer, rotation to set, click).
    pub frame_tick: fn(&FrameFacts, i32, i32) -> (i32, Option<i32>, bool),
    /// `StandVariant::hidden` for the variant the facts describe.
    pub stand_hidden: fn(&StandFacts, bool) -> bool,
}

static PROVIDER: OnceLock<Provider> = OnceLock::new();

pub fn set_provider(provider: Provider) -> Result<(), String> {
    PROVIDER.set(provider).map_err(|_| "hanging variant provider already installed".into())
}

pub fn active() -> bool {
    PROVIDER.get().is_some()
}

pub fn frame_for_item(item: &str) -> Option<FrameFacts> {
    PROVIDER.get().and_then(|provider| (provider.frame_for_item)(item))
}

pub fn frame_for_variant(variant: i32) -> Option<FrameFacts> {
    PROVIDER.get().and_then(|provider| (provider.frame_for_variant)(variant))
}

pub fn stand_for_item(item: &str) -> Option<StandFacts> {
    PROVIDER.get().and_then(|provider| (provider.stand_for_item)(item))
}

pub fn stand_for_variant(variant: i32) -> Option<StandFacts> {
    PROVIDER.get().and_then(|provider| (provider.stand_for_variant)(variant))
}

pub fn frame_rotation(facts: &FrameFacts, rotation_in: i32) -> i32 {
    PROVIDER.get().map_or(rotation_in, |provider| (provider.frame_rotation)(facts, rotation_in))
}

pub fn frame_analog(facts: &FrameFacts, filled: bool, rotation: i32) -> i32 {
    PROVIDER.get().map_or(0, |provider| (provider.frame_analog)(facts, filled, rotation))
}

pub fn frame_tick(facts: &FrameFacts, timer: i32, rotation: i32) -> (i32, Option<i32>, bool) {
    PROVIDER.get().map_or((timer, None, false), |provider| (provider.frame_tick)(facts, timer, rotation))
}

pub fn stand_hidden(facts: &StandFacts, any_equipment: bool) -> bool {
    PROVIDER.get().map_or(false, |provider| (provider.stand_hidden)(facts, any_equipment))
}
