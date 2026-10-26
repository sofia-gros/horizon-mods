//! FloatToolStat arithmetic ported from SlimeKnights (MIT); see third_party.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
pub struct Accumulator {
    base: f32,
    add: f32,
    percent: f32,
    multiply: f32,
}

impl Accumulator {
    pub fn new(default: f32) -> Self {
        Self {
            base: default,
            add: 0.0,
            percent: 0.0,
            multiply: 1.0,
        }
    }
    pub fn update(&mut self, value: f32) {
        self.add += value;
        self.base = 0.0;
    }
    // Java's compound assignment computes in double, then narrows to float.
    pub fn add(&mut self, value: f64) {
        self.add = (f64::from(self.add) + value) as f32;
    }
    pub fn percent(&mut self, value: f64) {
        self.percent = (f64::from(self.percent) + value) as f32;
    }
    pub fn multiply(&mut self, value: f64) {
        self.multiply = (f64::from(self.multiply) * value) as f32;
    }
    pub fn build(self) -> f32 {
        (self.base + self.add) * (1.0 + self.percent) * self.multiply
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    pub durability: f32,
    pub mining_speed: f32,
    pub attack_damage: f32,
    pub attack_speed: f32,
    pub harvest_tier: String,
}

impl Stats {
    pub fn durability(&self) -> Result<u32, String> {
        if !self.durability.is_finite()
            || self.durability < 1.0
            || self.durability > i32::MAX as f32
        {
            return Err("invalid durability".into());
        }
        // StatsNBT.getInt uses Number.intValue(), not rounding to nearest.
        Ok((self.durability as i32) as u32)
    }
    pub fn validate(&self) -> Result<(), String> {
        self.durability()?;
        for (name, value, min, max) in [
            ("mining_speed", self.mining_speed, 0.1, 2048.0),
            ("attack_damage", self.attack_damage, 0.0, 2048.0),
            ("attack_speed", self.attack_speed, 0.0, 1024.0),
        ] {
            if !value.is_finite() || value < min || value > max {
                return Err(format!("invalid {name}"));
            }
        }
        Ok(())
    }
}
