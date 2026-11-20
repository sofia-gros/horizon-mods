//! Fluids a mod defines: their id and how they look and behave (Fabric: `FluidVariantAttributes`; Forge: `FluidType`). The fluid
//! itself is data here: tanks hold it by id (see `BlockEntityType::fluid_tanks`), screens and renderers draw it with its colour.
//!
//! Horizon has no fluid kind for a mod. A definition can ask for source and flowing blocks in the world (`with_world_blocks`) and a bucket
//! (`with_bucket`): PlutoLib then runs the block's flow and the bucket's placing and picking up itself (see `fluid_world_api_v1`), on a block
//! the mod declares in its content files. Without them a fluid exists only in tanks.

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

/// How a fluid flows in the world (Forge: `ForgeFlowingFluid.Properties`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluidFlow {
    /// Game ticks between two steps of the flow (`tickRate`; water 5, lava 30 in the overworld).
    pub tick_delay: u16,
    /// Levels lost for each block the fluid flows (`levelDecreasePerBlock`; the source is 8, so 1 flows seven blocks and 8 none).
    pub level_drop: u8,
    /// How many blocks away the fluid looks for a place to go down when it has to choose a direction (`slopeFindDistance`).
    pub slope_distance: u8,
}

impl FluidFlow {
    /// The flow of an ordinary fluid: a step every 5 game ticks, one level lost per block and a way down looked for four blocks away.
    pub const fn standard() -> Self {
        FluidFlow { tick_delay: 5, level_drop: 1, slope_distance: 4 }
    }
}

impl Default for FluidFlow {
    fn default() -> Self {
        Self::standard()
    }
}

/// A fluid a mod defines.
#[derive(Clone, Debug, PartialEq)]
pub struct FluidDef {
    /// The fluid's id, such as `tconstruct:molten_iron`.
    pub id: Identifier,
    /// How it looks and behaves.
    pub attributes: FluidAttributes,
    /// The fluid has a block in the world with a source (`level` 0) and flowing cells, which PlutoLib flows. The mod declares the block.
    pub world_blocks: bool,
    /// The fluid has a bucket item (`<id>_bucket`) that holds 1000 and places or picks up the source when the fluid has world blocks.
    pub bucket: bool,
    /// How the fluid flows.
    pub flow: FluidFlow,
    /// The sound event a bucket makes taking the fluid up; `minecraft:item.bucket.fill` when none.
    pub bucket_fill_sound: Option<String>,
    /// The sound event a bucket makes pouring the fluid out; `minecraft:item.bucket.empty` when none.
    pub bucket_empty_sound: Option<String>,
}

impl FluidDef {
    /// A fluid with water's attributes, no world blocks and no bucket.
    pub fn new(id: Identifier) -> Self {
        Self { id, attributes: FluidAttributes::default(), world_blocks: false, bucket: false, flow: FluidFlow::standard(), bucket_fill_sound: None, bucket_empty_sound: None }
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

    /// The same fluid, with source and flowing blocks in the world. Registering it gives the block (the fluid's own id, declared in the
    /// mod's content with a `level` state) its flow behaviour.
    pub fn with_world_blocks(mut self) -> Self {
        self.world_blocks = true;
        self
    }

    /// The same fluid, with a bucket `<id>_bucket` (declared in the mod's content): a container of 1000 millibuckets that becomes an empty
    /// bucket, and, with world blocks, places and picks up the source.
    pub fn with_bucket(mut self) -> Self {
        self.bucket = true;
        self
    }

    /// The same fluid, flowing a step every `tick_delay` game ticks, losing `level_drop` levels a block and looking `slope_distance` blocks for
    /// a way down.
    pub fn flow(mut self, tick_delay: u16, level_drop: u8, slope_distance: u8) -> Self {
        self.flow = FluidFlow { tick_delay, level_drop, slope_distance };
        self
    }

    /// The same fluid, with the sound events its bucket makes taking it up and pouring it out.
    pub fn bucket_sounds(mut self, fill: &str, empty: &str) -> Self {
        self.bucket_fill_sound = Some(fill.to_owned());
        self.bucket_empty_sound = Some(empty.to_owned());
        self
    }
}

static FLUIDS: Mutex<Option<HashMap<String, FluidDef>>> = Mutex::new(None);

/// Owned fluid membership for one server reload: native registry IDs plus the
/// fluids defined through PlutoLib. This does not provide flowing-block support
/// or turn a later live attribute lookup into a same-generation lookup.
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct FluidRegistrySnapshot { ids:std::sync::Arc<std::collections::BTreeSet<Identifier>> }
impl FluidRegistrySnapshot {
    /// Whether this captured registry contains the fluid ID.
    pub fn contains(&self,id:&Identifier)->bool { self.ids.contains(id) }
    pub(crate) fn capture(base:Option<&[String]>)->Result<Option<Self>,String> {
        let Some(base)=base else { return Ok(None) };
        let mut ids=std::collections::BTreeSet::new();
        for id in base {
            let id=Identifier::parse(id).map_err(|e|e.to_string())?;
            if !ids.insert(id) { return Err("duplicate native fluid registry ID".into()); }
        }
        let defined=FLUIDS.lock().map_err(|_|"fluid definitions registry poisoned")?;
        if let Some(defined)=defined.as_ref() { ids.extend(defined.values().map(|def|def.id.clone())); }
        Ok(Some(Self { ids:std::sync::Arc::new(ids) }))
    }
}
/// The lookup `ResourceManagerHelper::reload_registry("minecraft:fluid")` returns: the native fluid IDs joined with the fluids defined
/// through PlutoLib, and the loaded fluid tags. A tag member that is no known fluid (a pack naming a fluid no mod defines, or a mod
/// fluid that is not defined) is left out of its tag, as the host's own tag loader treats unknown entries; the tag stays present.
/// Malformed IDs reject the whole lookup, so a bad capture is unavailable rather than silently partial.
pub(crate) fn reload_lookup(base:&[String],tags:&[(String,Vec<String>)])->Result<super::resource_loader_v0::RegistryLookup,String> {
    let mut entries:std::collections::BTreeSet<String>=base.iter().cloned().collect();
    {
        let defined=FLUIDS.lock().map_err(|_|"fluid definitions registry poisoned")?;
        if let Some(defined)=defined.as_ref() { entries.extend(defined.values().map(|def|def.id.to_string())); }
    }
    let entry_list:Vec<String>=entries.iter().cloned().collect();
    let known_tags:Vec<(String,Vec<String>)>=tags.iter().map(|(tag,members)|
        (tag.clone(),members.iter().filter(|member|entries.contains(*member)).cloned().collect())).collect();
    super::resource_loader_v0::RegistryLookup::from_parts(&entry_list,&known_tags)
}
thread_local! { static CURRENT_FLUIDS:std::cell::RefCell<Vec<Option<FluidRegistrySnapshot>>>=const { std::cell::RefCell::new(Vec::new()) }; }
pub(crate) fn with_fluid_registry<T>(snapshot:Option<FluidRegistrySnapshot>,action:impl FnOnce()->T)->T {
    struct Scope;
    impl Drop for Scope { fn drop(&mut self) { CURRENT_FLUIDS.with(|v| { v.borrow_mut().pop(); }); } }
    CURRENT_FLUIDS.with(|v|v.borrow_mut().push(snapshot));
    let _scope=Scope;
    action()
}

/// Registers fluids (Fabric: `Registry.register(Registries.FLUID, ...)`).
pub struct FluidRegistry;

impl FluidRegistry {
    /// This server JSON callback's owned fluid membership, or None when the
    /// native registry export/scope is unavailable. Never substitutes an empty
    /// native registry or hardcoded vanilla IDs. Retain it with parsed tag data.
    pub fn snapshot()->Option<FluidRegistrySnapshot> {
        CURRENT_FLUIDS.with(|v|v.borrow().last().cloned().flatten())
    }

    /// Defines a fluid. Registering the same id again replaces the earlier definition.
    pub fn register(def: FluidDef) {
        let (world_blocks, bucket) = (def.world_blocks, def.bucket);
        let behaviour = def.clone();
        if let Ok(mut all) = FLUIDS.lock() {
            all.get_or_insert_with(HashMap::new).insert(def.id.to_string(), def);
        }
        if world_blocks {
            super::fluid_world_api_v1::FluidWorld::register_behaviour(&behaviour);
        }
        if bucket {
            super::fluid_world_api_v1::register_bucket(&behaviour);
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
    fn reload_membership_requires_native_capture_and_does_not_refresh_retained_fluid_ids() {
        let old_id=Identifier::parse("test:snapshot_fluid_old").unwrap();
        let late_id=Identifier::parse("test:snapshot_fluid_late").unwrap();
        FluidRegistry::register(FluidDef::new(old_id.clone()));
        assert!(FluidRegistrySnapshot::capture(None).unwrap().is_none(),"defined mod fluids cannot stand in for unavailable native membership");
        let base=vec!["minecraft:water".to_owned()];
        let original=FluidRegistrySnapshot::capture(Some(&base)).unwrap().unwrap();
        assert!(original.contains(&old_id));
        assert!(original.contains(&Identifier::parse("minecraft:water").unwrap()));
        assert!(!original.contains(&late_id));
        assert!(FluidRegistry::snapshot().is_none());
        let kept=with_fluid_registry(Some(original.clone()),|| {
            let kept=FluidRegistry::snapshot().unwrap();
            FluidRegistry::register(FluidDef::new(late_id.clone()));
            let newer=FluidRegistrySnapshot::capture(Some(&base)).unwrap().unwrap();
            with_fluid_registry(Some(newer),||assert!(FluidRegistry::snapshot().unwrap().contains(&late_id)));
            let failed=std::panic::catch_unwind(||with_fluid_registry(None,|| {
                assert!(FluidRegistry::snapshot().is_none()); panic!("nested fluid reload failed");
            }));
            assert!(failed.is_err()); assert_eq!(FluidRegistry::snapshot(),Some(original.clone()));
            kept
        });
        assert!(!kept.contains(&late_id)); assert!(FluidRegistry::snapshot().is_none());
        assert!(FluidRegistrySnapshot::capture(Some(&["bad id".into()])).is_err());
        assert!(FluidRegistrySnapshot::capture(Some(&[base[0].clone(),base[0].clone()])).is_err());
    }

    #[test]
    fn the_fluid_lookup_joins_native_and_defined_fluids_with_tags_and_drops_unknown_members() {
        let id=|text:&str|Identifier::parse(text).unwrap();
        FluidRegistry::register(FluidDef::new(id("test:lookup_fluid")));
        let base=vec!["minecraft:water".to_owned(),"minecraft:lava".to_owned()];
        let tags=vec![
            ("test:molten".to_owned(),vec!["test:lookup_fluid".to_owned(),"test:lookup_flowing_fluid".to_owned(),"minecraft:lava".to_owned()]),
            ("minecraft:water".to_owned(),vec!["minecraft:water".to_owned()]),
            ("test:empty".to_owned(),Vec::new()),
        ];
        let lookup=reload_lookup(&base,&tags).unwrap();
        assert!(lookup.contains(&id("test:lookup_fluid")) && lookup.contains(&id("minecraft:water")));
        assert!(!lookup.contains(&id("test:lookup_flowing_fluid")));
        let molten:Vec<_>=lookup.get_tag(&id("test:molten")).unwrap().iter().map(ToString::to_string).collect();
        assert_eq!(molten,["minecraft:lava","test:lookup_fluid"],"the flowing variant nobody defined is not a member");
        assert!(lookup.get_tag(&id("test:empty")).unwrap().is_empty(),"present but empty differs from absent");
        assert!(lookup.get_tag(&id("test:absent")).is_none());
        assert!(reload_lookup(&base,&[("bad tag".to_owned(),Vec::new())]).is_err());
        assert!(reload_lookup(&["bad id".to_owned()],&[]).is_err());
    }

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
