//! OvergrowthModule, from SlimeKnights (MIT; see LICENSE): once a second, a carried tool with room in
//! its overslime bar has a leveling chance to grow a point of overslime back. The growth is worked out
//! here over the tool's data exactly as the module runs it; the port has no inventory tick to drive it
//! (PlutoLib hears a player's tick only for the armor they wear, never for the tools in their inventory),
//! so nothing calls it in game yet.
use crate::{modifier_entries::Entry, overslime, tool_stack::Environment};
use pluto_lib::NbtCompound;
use serde_json::{Map, Value};

/// OvergrowthModule: the overslime bar grows back at a chance per level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Overgrowth { chance_flat: f32, chance_each: f32 }

impl Overgrowth {
    /// The chance is a required leveling value (OvergrowthModule.CHANCE_FIELD).
    pub fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let chance = fields.get("chance").and_then(Value::as_object).ok_or("chance must be a leveling value")?;
        let number = |key: &str| -> Result<f32, String> {
            match chance.get(key).filter(|value| !value.is_null()) {
                None => Ok(0.0),
                Some(value) => value.as_f64().map(|value| value as f32).ok_or_else(|| format!("chance {key} must be a number")),
            }
        };
        Ok(Self { chance_flat: number("flat")?, chance_each: number("each_level")? })
    }

    /// The chance for one second's tick at the entry's effective level.
    pub fn chance(&self, entry: &Entry) -> f32 { self.chance_flat + self.chance_each * entry.effective_level() }

    /// InventoryTickModifierHook.onInventoryTick for one second's tick of the holder: grows a point of
    /// overslime when the module's condition held, the bar has room and the roll lands (a roll in [0, 1)
    /// below the chance). Returns whether the tool changed. The tick's own gating (once a second, on the
    /// server, not while the holder uses this stack) is the driver's, in `tool_stack::overgrowth_tick`.
    pub fn grow(&self, entry: &Entry, condition: bool, nbt: &mut NbtCompound, env: &Environment, roll: f32) -> bool {
        if !condition { return false; }
        if overslime::amount(nbt) >= overslime::capacity(nbt) { return false; }
        if roll >= self.chance(entry) { return false; }
        overslime::add_amount(nbt, env, 1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{materials::Definitions as Materials, modifiers::Registry, tool_stack::{TAG_PERSISTENT, TAG_STATS}};
    use pluto_lib::{Identifier, Nbt, NbtCompound};
    use serde_json::json;

    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:overgrowth").unwrap(), level) }

    /// A tool with an overslime capacity of 10 and the given amount already on it.
    fn tool(amount: i32) -> NbtCompound {
        let mut nbt = NbtCompound::new();
        let mut stats = NbtCompound::new();
        stats.insert("tconstruct:overslime".into(), Nbt::Float(10.0));
        nbt.insert(TAG_STATS.into(), Nbt::Compound(stats));
        if amount > 0 {
            let mut persistent = NbtCompound::new();
            persistent.insert("tconstruct:overslime".into(), Nbt::Int(amount));
            nbt.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        }
        nbt
    }

    #[test]
    fn the_chance_scales_with_the_level() {
        // The shipped overgrowth module: 0.05 per level, no flat part.
        let overgrowth = Overgrowth::decode(json!({"chance": {"each_level": 0.05}}).as_object().unwrap()).unwrap();
        assert!((overgrowth.chance(&entry(1)) - 0.05).abs() < 1e-6);
        assert!((overgrowth.chance(&entry(3)) - 0.15).abs() < 1e-6);
        assert!(Overgrowth::decode(json!({}).as_object().unwrap()).is_err(), "the chance is required");
    }

    #[test]
    fn a_landing_roll_grows_a_point_and_a_full_bar_or_a_miss_grows_nothing() {
        let overgrowth = Overgrowth::decode(json!({"chance": {"each_level": 0.05}}).as_object().unwrap()).unwrap();
        let materials = Materials::default();
        let modifiers = Registry::default();
        let env = Environment { definition: None, has_materials: false, materials: &materials, modifiers: &modifiers, item_tags_loaded: false, item: None, item_tags: None };
        let mut nbt = tool(0);
        assert!(overgrowth.grow(&entry(2), true, &mut nbt, &env, 0.05), "a roll below the chance of 0.1 grows");
        assert_eq!(overslime::amount(&nbt), 1);
        assert!(!overgrowth.grow(&entry(2), true, &mut nbt, &env, 0.5), "a roll above the chance does not");
        assert!(!overgrowth.grow(&entry(2), false, &mut nbt, &env, 0.0), "a failed condition does not");
        let mut full = tool(10);
        assert!(!overgrowth.grow(&entry(2), true, &mut full, &env, 0.0), "a full bar does not");
        assert_eq!(overslime::amount(&full), 10);
    }
}
