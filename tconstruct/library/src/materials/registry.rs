//! `library.materials.MaterialRegistry` / `IMaterialRegistry` / `MaterialRegistryImpl`: the three managers behind one set of lookups, and
//! `definition.MaterialVariant` / `LazyMaterial`: a material named by id and resolved when it is asked for.
use std::cmp::Ordering;
use std::sync::Arc;

use super::ids::{MaterialId, MaterialStatsId, MaterialVariantId};
use super::manager::MaterialManager;
use super::material::Material;
use super::stats::{MaterialStatType, MaterialStats, MaterialStatsManager};
use super::traits::{MaterialTraitsManager, ModifierEntry};
use crate::resource::ResourceId;

/// The materials, their stats and their traits.
#[derive(Default)]
pub struct MaterialRegistry {
    pub materials: MaterialManager,
    pub stats: MaterialStatsManager,
    pub traits: MaterialTraitsManager,
    unknown: Option<Material>,
}

impl MaterialRegistry {
    pub fn new() -> Self {
        Self { unknown: Some(Material::unknown()), ..Self::default() }
    }

    /// `resolve`: follows a redirect.
    pub fn resolve<'a>(&'a self, id: &'a MaterialId) -> &'a MaterialId {
        self.materials.resolve_redirect(id)
    }

    /// `getMaterial`: the material, or `IMaterial.UNKNOWN` when it is not loaded.
    pub fn material(&self, id: &MaterialId) -> &Material {
        self.materials.material(id).unwrap_or_else(|| self.unknown.as_ref().expect("the registry is made with new()"))
    }

    pub fn visible_materials(&self) -> Vec<&Material> {
        self.materials.visible_materials()
    }

    pub fn is_in_tag(&self, id: &MaterialId, tag: &ResourceId) -> bool {
        self.materials.is_in(id, tag)
    }

    pub fn tag_values(&self, tag: &ResourceId) -> &[MaterialId] {
        self.materials.tag(tag)
    }

    /// `registerStatType(type)`.
    pub fn register_stat_type(&mut self, kind: MaterialStatType) -> Result<(), String> {
        self.stats.register(kind)
    }

    /// `registerStatType(type, fallback)`: also lets the type use the fallback's traits when it has none of its own.
    pub fn register_stat_type_with_fallback(&mut self, kind: MaterialStatType, fallback: Option<MaterialStatsId>) -> Result<(), String> {
        let id = kind.id.clone();
        self.stats.register(kind)?;
        if let Some(fallback) = fallback {
            self.traits.register_fallback(id, fallback)?;
        }
        Ok(())
    }

    /// `getMaterialStats`.
    pub fn material_stats(&self, material: &MaterialId, stat: &MaterialStatsId) -> Option<&Arc<dyn MaterialStats>> {
        self.stats.stats(material, stat)
    }

    /// `getStatsOrDefault`.
    pub fn stats_or_default(&self, material: &MaterialId, stat: &MaterialStatsId) -> Option<Arc<dyn MaterialStats>> {
        self.stats.stats_or_default(material, stat)
    }

    pub fn default_traits(&self, material: &MaterialId) -> &[ModifierEntry] {
        self.traits.default_traits(material)
    }

    pub fn has_unique_traits(&self, material: &MaterialId, stat: &MaterialStatsId) -> bool {
        self.traits.has_unique_traits(material, stat)
    }

    pub fn traits(&self, material: &MaterialId, stat: &MaterialStatsId) -> &[ModifierEntry] {
        self.traits.traits(material, stat)
    }
}

/// `MaterialVariant`: a material and variant named by id; `get` looks the material up when needed. Equality and hashing are by id and
/// variant, as in the original.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialVariant {
    pub variant: MaterialVariantId,
}

impl MaterialVariant {
    pub fn of(variant: MaterialVariantId) -> Self {
        Self { variant }
    }

    /// `MaterialVariant.UNKNOWN`.
    pub fn unknown() -> Self {
        Self { variant: MaterialVariantId::Plain(MaterialId::unknown()) }
    }

    pub fn id(&self) -> &MaterialId {
        self.variant.id()
    }

    /// `LazyMaterial.get`.
    pub fn get<'a>(&self, registry: &'a MaterialRegistry) -> &'a Material {
        registry.material(self.id())
    }

    pub fn matches_variant(&self, other: &MaterialVariantId) -> bool {
        self.variant.matches_variant(other)
    }

    pub fn same_variant(&self, other: &MaterialVariantId) -> bool {
        self.variant.same_variant(other)
    }

    /// `compareTo`: different materials by `IMaterial` order; the same material by variant name.
    pub fn compare(&self, other: &Self, registry: &MaterialRegistry) -> Ordering {
        if self.id() != other.id() {
            self.get(registry).cmp(other.get(registry))
        } else {
            self.variant.variant().cmp(other.variant.variant())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::EmptyContext;
    use serde_json::json;

    fn registry() -> MaterialRegistry {
        let files = vec![
            (ResourceId::try_parse("tconstruct:wood").unwrap(), json!({"craftable": true, "tier": 0, "sortOrder": 0})),
            (ResourceId::try_parse("tconstruct:iron").unwrap(), json!({"craftable": true, "tier": 2, "sortOrder": 1})),
            (ResourceId::try_parse("tconstruct:alias").unwrap(), json!({"redirect": [{"id": "tconstruct:iron"}]})),
        ];
        let (materials, problems) = MaterialManager::load(&files, &[], &EmptyContext);
        assert!(problems.is_empty());
        let mut registry = MaterialRegistry::new();
        registry.materials = materials;
        registry
    }

    fn id(text: &str) -> MaterialId {
        MaterialId::try_parse(text).unwrap()
    }

    #[test]
    fn an_unloaded_material_is_unknown() {
        let registry = registry();
        assert_eq!(registry.material(&id("tconstruct:wood")).tier, 0);
        let missing = registry.material(&id("tconstruct:nothing"));
        assert_eq!(missing.id, MaterialId::unknown());
        assert!(missing.hidden && !missing.craftable);
        assert_eq!(registry.resolve(&id("tconstruct:alias")).to_string(), "tconstruct:iron");
    }

    #[test]
    fn variants_compare_by_material_then_variant_name() {
        let registry = registry();
        let wood = MaterialVariant::of(MaterialVariantId::try_parse("tconstruct:wood#b").unwrap());
        let iron = MaterialVariant::of(MaterialVariantId::try_parse("tconstruct:iron").unwrap());
        assert_eq!(wood.compare(&iron, &registry), Ordering::Less, "tier 0 before tier 2");
        let wood_a = MaterialVariant::of(MaterialVariantId::try_parse("tconstruct:wood#a").unwrap());
        assert_eq!(wood.compare(&wood_a, &registry), Ordering::Greater);
        assert_ne!(wood, wood_a);
        assert_eq!(MaterialVariant::unknown().get(&registry).id, MaterialId::unknown());
    }

    #[test]
    fn a_stat_type_registered_with_a_fallback_registers_the_fallback_too() {
        use super::super::stats::singleton;
        use std::any::Any;
        struct Core(MaterialStatsId);
        impl MaterialStats for Core {
            fn id(&self) -> &MaterialStatsId {
                &self.0
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
        let mut registry = registry();
        let grip = MaterialStatsId::try_parse("tconstruct:grip").unwrap();
        let handle = MaterialStatsId::try_parse("tconstruct:handle").unwrap();
        registry.register_stat_type_with_fallback(singleton(Arc::new(Core(grip.clone()))), Some(handle.clone())).unwrap();
        assert!(registry.stats.stat_type(&grip).is_some());
        registry.traits.load(&[(ResourceId::try_parse("tconstruct:wood").unwrap(), json!({"perStat": {"tconstruct:handle": [{"name": "a:b"}]}}))]);
        assert!(registry.has_unique_traits(&id("tconstruct:wood"), &grip));
    }
}
