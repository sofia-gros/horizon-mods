//! Fluids a mod defines: their id and how they look and behave (Fabric: `FluidVariantAttributes`; Forge: `FluidType`). The fluid
//! itself is data here: tanks hold it by id (see `BlockEntityType::fluid_tanks`), screens and renderers draw it with its colour.
//!
//! Horizon does not yet have source or flowing blocks of a mod's fluid, nor buckets of one: a definition can say it wants them, which is
//! kept and does nothing until the game offers them (see the support matrix).

use std::collections::HashMap;
use std::sync::Mutex;

use super::base::Identifier;

/// How a fluid looks and behaves.
#[derive(Clone, Debug, PartialEq)]
pub struct FluidAttributes {
    /// Temperature in kelvin (water is 300, lava 1300).
    pub temperature: i32,
    /// Light level the fluid gives, 0 to 15.
    pub luminance: u8,
    /// Density; negative fluids rise (water is 1000, lava 3000).
    pub density: i32,
    /// Viscosity; the higher the slower it flows (water is 1000, lava 6000).
    pub viscosity: i32,
    /// Colour as `0xAARRGGBB`; white leaves the fluid texture as it is.
    pub color: u32,
}

impl Default for FluidAttributes {
    fn default() -> Self {
        Self { temperature: 300, luminance: 0, density: 1000, viscosity: 1000, color: 0xFFFF_FFFF }
    }
}

/// A fluid a mod defines.
#[derive(Clone, Debug, PartialEq)]
pub struct FluidDef {
    /// The fluid's id, such as `tconstruct:molten_iron`.
    pub id: Identifier,
    /// How it looks and behaves.
    pub attributes: FluidAttributes,
    /// The fluid wants a source and a flowing block in the world (stub: the game has none yet).
    pub world_blocks: bool,
    /// The fluid wants a bucket (stub: the game has none yet).
    pub bucket: bool,
}

impl FluidDef {
    /// A fluid with water's attributes, no world blocks and no bucket.
    pub fn new(id: Identifier) -> Self {
        Self { id, attributes: FluidAttributes::default(), world_blocks: false, bucket: false }
    }

    /// The key of the fluid's name in the language files: `fluid.<namespace>.<path>` (`fluid.tconstruct.molten_iron`).
    pub fn translation_key(&self) -> String {
        format!("fluid.{}.{}", self.id.namespace(), self.id.path())
    }

    /// The same fluid with the given temperature in kelvin.
    pub fn temperature(mut self, kelvin: i32) -> Self {
        self.attributes.temperature = kelvin;
        self
    }

    /// The same fluid giving the given light level (0 to 15).
    pub fn luminance(mut self, level: u8) -> Self {
        self.attributes.luminance = level.min(15);
        self
    }

    /// The same fluid with the given density.
    pub fn density(mut self, density: i32) -> Self {
        self.attributes.density = density;
        self
    }

    /// The same fluid with the given viscosity.
    pub fn viscosity(mut self, viscosity: i32) -> Self {
        self.attributes.viscosity = viscosity;
        self
    }

    /// The same fluid with the given colour (`0xAARRGGBB`).
    pub fn color(mut self, argb: u32) -> Self {
        self.attributes.color = argb;
        self
    }

    /// The same fluid, wanting source and flowing blocks in the world. Kept; does nothing until the game has mod fluid blocks.
    pub fn with_world_blocks(mut self) -> Self {
        self.world_blocks = true;
        self
    }

    /// The same fluid, wanting a bucket. Kept; does nothing until the game has mod fluid buckets.
    pub fn with_bucket(mut self) -> Self {
        self.bucket = true;
        self
    }
}

static FLUIDS: Mutex<Option<HashMap<String, FluidDef>>> = Mutex::new(None);

/// Registers fluids (Fabric: `Registry.register(Registries.FLUID, ...)`).
pub struct FluidRegistry;

impl FluidRegistry {
    /// Defines a fluid. Registering the same id again replaces the earlier definition.
    pub fn register(def: FluidDef) {
        if let Ok(mut all) = FLUIDS.lock() {
            all.get_or_insert_with(HashMap::new).insert(def.id.to_string(), def);
        }
    }

    /// The definition of the fluid `id` (such as `tconstruct:molten_iron`), if a mod defined it.
    pub fn get(id: &str) -> Option<FluidDef> {
        FLUIDS.lock().ok()?.as_ref()?.get(id).cloned()
    }

    /// The fluid's name in the language in use (see `I18n`); the translation key itself if nobody translated it, and `None` if no mod
    /// defined the fluid.
    pub fn display_name(id: &str) -> Option<String> {
        Self::get(id).map(|def| super::i18n_api::I18n::translate(&def.translation_key()))
    }

    /// The fluid's name in `language`, whatever the game is set to; `None` if no mod defined the fluid.
    pub fn display_name_in(language: &str, id: &str) -> Option<String> {
        Self::get(id).map(|def| super::i18n_api::I18n::translate_in(language, &def.translation_key()))
    }

    /// Every defined fluid, sorted by id.
    pub fn registered() -> Vec<FluidDef> {
        let mut all: Vec<FluidDef> = FLUIDS.lock().ok().and_then(|m| m.as_ref().map(|m| m.values().cloned().collect())).unwrap_or_default();
        all.sort_by(|a, b| a.id.to_string().cmp(&b.id.to_string()));
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_defined_fluid_keeps_its_attributes_replaces_on_reregistration_and_unknown_ids_are_none() {
        let id = Identifier::parse("test:molten_thing").unwrap();
        FluidRegistry::register(FluidDef::new(id.clone()).temperature(1100).luminance(40).density(2000).viscosity(10000).color(0xFFAA5500).with_bucket());
        let got = FluidRegistry::get("test:molten_thing").unwrap();
        assert_eq!(got.attributes, FluidAttributes { temperature: 1100, luminance: 15, density: 2000, viscosity: 10000, color: 0xFFAA5500 });
        assert!(got.bucket && !got.world_blocks);
        FluidRegistry::register(FluidDef::new(id).with_world_blocks());
        assert_eq!(FluidRegistry::get("test:molten_thing").unwrap().attributes, FluidAttributes::default());
        assert!(FluidRegistry::get("test:unknown").is_none());
        assert!(FluidRegistry::registered().iter().any(|f| f.id.path() == "molten_thing"));
    }
}
