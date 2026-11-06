// Explicit, bounded requests for staged native status clouds. No actor is inferred.
use horizon_client::pluto_potions;
use std::{collections::BTreeMap, fmt};

pub const MAX_EFFECTS: usize = 256;
pub const MAX_LABEL_BYTES: usize = 4096;

/// Requested attribution, not evidence of the caller's authority or actor identity.
/// The caller must establish that binding separately before any world insertion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attribution {
    Unowned,
    Player { guest_connection: Option<u32> },
    Mob { entity_id: u32 },
}

/// A deliberately small label surface; no implicit cloud or player name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Label {
    Literal(String),
    Translation(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effect {
    pub id: String,
    pub duration_ticks: i32,
    pub amplifier: i32,
    pub visible: bool,
    pub ambient: bool,
}

/// All lifecycle choices belong to the consumer. There is intentionally no Default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timing {
    pub age: i32,
    /// -1 means indefinite; zero means expiry at the native lifetime check.
    pub duration_ticks: i32,
    pub wait_ticks: i32,
    pub reapplication_ticks: i32,
    pub duration_on_use: i32,
    pub effect_duration_scale: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Descriptor {
    pub position: [f64; 3],
    pub radius: f32,
    pub radius_per_tick: f32,
    pub radius_on_use: f32,
    pub timing: Timing,
    /// Explicit RGB input to validate(), without an alpha byte. The consuming
    /// Validated::with_native_effect_color operation instead stores the exact
    /// native color-mix u32 result, including its native overflow behavior.
    pub color_rgb: u32,
    pub label: Label,
    pub attribution: Attribution,
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidPosition,
    InvalidRadius,
    InvalidTiming,
    InvalidColor,
    InvalidLabel,
    EffectBudget,
    UnknownEffect { index: usize },
    InvalidEffect { index: usize },
    ConflictingFlags { index: usize },
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid cloud descriptor: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Catalog-resolved facts in caller order. Duration and amplifier are not scaled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedEffect {
    pub raw_id: &'static str,
    pub duration_ticks: i32,
    pub amplifier: i32,
    pub visible: bool,
    pub ambient: bool,
}

/// Immutable, validated request. It is not registered, spawned or actor-authorized.
#[derive(Clone, Debug)]
pub struct Validated {
    descriptor: Descriptor,
    effects: Vec<ResolvedEffect>,
}
impl Validated {
    pub fn descriptor(&self) -> &Descriptor { &self.descriptor }
    pub fn effects(&self) -> &[ResolvedEffect] { &self.effects }

    /// Opt into the native mix of the validated visible effects, in caller
    /// order. This accepts no raw color value and grants no actor authority.
    /// Native u32 arithmetic (including its profile-dependent overflow behavior)
    /// is preserved; the resulting bits need not satisfy the separate explicit
    /// RGB-input restriction. Compute this before native construction/ID work.
    pub fn with_native_effect_color(mut self) -> Self {
        self.descriptor.color_rgb = pluto_potions::effects_color(self.effects.iter()
            .filter(|effect| effect.visible).map(|effect| (effect.raw_id, effect.amplifier)));
        self
    }
}

/// Resolve catalog identities and reject unsupported/nonfinite values before any
/// native object construction. No world read/write, entity ID, default owner,
/// application, filtering, sorting, scaling or color averaging is performed.
pub fn validate(descriptor: Descriptor) -> Result<Validated, Error> {
    validate_with_effect_budget(descriptor, MAX_EFFECTS)
}

/// The same validation with an explicit finite effect-count budget. The default
/// entry keeps its 256 limit. This does not authorize a cloud or allocate IDs.
/// A caller preserving an already admitted owned sequence may supply its exact
/// length; it remains responsible for that input's admission and memory cost.
/// Budgets impossible for either owned vector layout reject before validation.
/// No sorting, duplicate collapse, filtering or partial validation is performed.
pub fn validate_with_effect_budget(descriptor: Descriptor, effect_budget: usize)
    -> Result<Validated, Error> {
    for element_bytes in [std::mem::size_of::<Effect>(), std::mem::size_of::<ResolvedEffect>()] {
        if effect_budget.checked_mul(element_bytes).is_none_or(|bytes| bytes > isize::MAX as usize) {
            return Err(Error::EffectBudget);
        }
    }
    if descriptor.position.iter().any(|n| !n.is_finite()
        || *n < f64::from(i32::MIN) || *n >= f64::from(i32::MAX) + 1.0) {
        return Err(Error::InvalidPosition);
    }
    if !descriptor.radius.is_finite() || !(0.0..=32.0).contains(&descriptor.radius)
        || !descriptor.radius_per_tick.is_finite() || !descriptor.radius_on_use.is_finite() {
        return Err(Error::InvalidRadius);
    }
    let timing = descriptor.timing;
    if timing.age < 0 || timing.duration_ticks < -1 || timing.wait_ticks < 0
        || timing.reapplication_ticks < 0 || !timing.effect_duration_scale.is_finite()
        || timing.effect_duration_scale < 0.0 {
        return Err(Error::InvalidTiming);
    }
    if descriptor.color_rgb > 0x00ff_ffff { return Err(Error::InvalidColor); }
    let label = match &descriptor.label { Label::Literal(value) | Label::Translation(value) => value };
    if label.len() > MAX_LABEL_BYTES || matches!(&descriptor.label, Label::Translation(key) if key.is_empty()) {
        return Err(Error::InvalidLabel);
    }
    if descriptor.effects.is_empty() || descriptor.effects.len() > effect_budget {
        return Err(Error::EffectBudget);
    }
    let mut effects = Vec::with_capacity(descriptor.effects.len());
    let mut flags = BTreeMap::new();
    for (index, effect) in descriptor.effects.iter().enumerate() {
        if effect.duration_ticks < -1 || !(0..=255).contains(&effect.amplifier) {
            return Err(Error::InvalidEffect { index });
        }
        let Some(def) = pluto_potions::effect(&effect.id) else { return Err(Error::UnknownEffect { index }); };
        // The native cloud stores visibility/ambient flags by identity rather
        // than per duplicate entry. Refuse requests it cannot represent exactly.
        let current = (effect.visible, effect.ambient);
        if flags.insert(def.raw_id, current).is_some_and(|prior| prior != current) {
            return Err(Error::ConflictingFlags { index });
        }
        effects.push(ResolvedEffect { raw_id: def.raw_id, duration_ticks: effect.duration_ticks,
            amplifier: effect.amplifier, visible: effect.visible, ambient: effect.ambient });
    }
    Ok(Validated { descriptor, effects })
}
