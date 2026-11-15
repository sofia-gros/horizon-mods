//! Entity-type declarations, separate from entity load/damage events.
//! The derived-type convenience API reuses an existing vanilla implementation;
//! it never approximates a missing implementation with an unrelated mob.
use std::collections::BTreeMap;
use std::sync::Mutex;
use super::base::Identifier;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DerivedEntityType {
    pub id: Identifier,
    pub factory: Identifier,
    pub width: f32,
    pub height: f32,
    pub tracking_chunks: u32,
    pub velocity_updates: bool,
}
#[derive(Default)]
struct Registry { frozen: bool, types: BTreeMap<String, DerivedEntityType> }
impl Registry {
    fn register(&mut self, definition: DerivedEntityType) -> Result<(), String> {
        if self.frozen { return Err("entity types have already been collected".into()); }
        if definition.id.to_string().starts_with("minecraft:") {
            return Err("derived registration cannot replace a vanilla entity type".into());
        }
        if !definition.factory.to_string().starts_with("minecraft:") {
            return Err("derived factory must name an existing vanilla entity type".into());
        }
        if !definition.width.is_finite() || definition.width <= 0.0 || !definition.height.is_finite() || definition.height <= 0.0
            || definition.tracking_chunks == 0 || definition.tracking_chunks > i32::MAX as u32 {
            return Err("entity dimensions and tracking range must be finite and positive".into());
        }
        let key = definition.id.to_string();
        if self.types.contains_key(&key) { return Err(format!("entity type {key} already registered")); }
        self.types.insert(key, definition);
        Ok(())
    }
    fn snapshot(&mut self) -> Vec<DerivedEntityType> {
        self.frozen = true;
        self.types.values().cloned().collect()
    }
}
static TYPES: Mutex<Registry> = Mutex::new(Registry { frozen: false, types: BTreeMap::new() });

/// Registers entity types independently of content-file template whitelists.
/// This is a convenience for deriving from an existing Minecraft entity class,
/// not an API for constructing arbitrary Rust entities. It records declarations
/// with a source-wired symbolic export adapter; native registry/loader and
/// runtime policy validation remain an explicit stub.
pub struct EntityTypeRegistry;
impl EntityTypeRegistry {
    /// Declare a custom type reusing `factory`'s vanilla runtime implementation.
    /// Category and unspecified defaults come from the factory. Dimensions are
    /// fixed width/height in blocks; tracking is in chunks, not blocks.
    /// Duplicate, invalid and post-collection calls fail without replacement.
    /// The future native adapter must validate factory existence, resource model
    /// availability and all registration consumers, not just a type-name list.
    pub fn register_derived(id: Identifier, factory: Identifier, width: f32, height: f32,
        tracking_chunks: u32, velocity_updates: bool) -> Result<(), String> {
        TYPES.lock().map_err(|_| "entity type registry poisoned")?
            .register(DerivedEntityType { id, factory, width, height, tracking_chunks, velocity_updates })
    }
}

pub(crate) fn derived_entity_types() -> Result<Vec<DerivedEntityType>, String> {
    Ok(TYPES.lock().map_err(|_| "entity type registry poisoned")?.snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definition(id: &str) -> DerivedEntityType {
        DerivedEntityType { id: Identifier::parse(id).unwrap(), factory: Identifier::parse("minecraft:slime").unwrap(),
            width: 2.04, height: 2.04, tracking_chunks: 20, velocity_updates: true }
    }
    #[test]
    fn derived_types_keep_distinct_ids_owned_properties_and_freeze_without_replacement() {
        let mut registry = Registry::default();
        let original = definition("example:sky");
        registry.register(original.clone()).unwrap();
        let mut duplicate = original.clone(); duplicate.factory = Identifier::parse("minecraft:zombie").unwrap();
        assert!(registry.register(duplicate).is_err());
        let mut other = definition("example:ender"); other.tracking_chunks = 32;
        registry.register(other.clone()).unwrap();
        let mut copy = registry.snapshot();
        assert_eq!(copy, [other, original.clone()]);
        copy[1].width = 99.0;
        assert_eq!(registry.snapshot()[1], original);
        assert!(registry.register(definition("example:late")).is_err());
    }
    #[test]
    fn invalid_dimensions_factories_and_vanilla_replacement_never_publish() {
        let mut registry = Registry::default();
        for value in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let mut d = definition("example:bad"); d.width = value;
            assert!(registry.register(d).is_err());
        }
        let mut d = definition("example:bad"); d.tracking_chunks = 0;
        assert!(registry.register(d).is_err());
        let mut d = definition("example:bad"); d.factory = Identifier::parse("example:unknown").unwrap();
        assert!(registry.register(d).is_err());
        assert!(registry.register(definition("minecraft:slime")).is_err());
        assert!(registry.snapshot().is_empty());
    }
}
