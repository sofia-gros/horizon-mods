//! `library.materials.definition.MaterialId`, `MaterialVariantId` (and `MaterialVariantIdImpl`) and `library.materials.stats.MaterialStatsId`.
use std::fmt;

use crate::resource::{is_valid_path, ResourceId};

/// The id of a material (`tconstruct:wood`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialId(ResourceId);

impl MaterialId {
    /// `MaterialId.UNKNOWN`.
    pub fn unknown() -> Self {
        Self(ResourceId::try_new("tconstruct", "unknown").expect("valid id"))
    }

    /// `MaterialId.tryParse`: the default namespace is `minecraft`.
    pub fn try_parse(text: &str) -> Option<Self> {
        ResourceId::try_parse(text).map(Self)
    }

    pub fn try_new(namespace: &str, path: &str) -> Option<Self> {
        ResourceId::try_new(namespace, path).map(Self)
    }

    pub fn from_resource(id: ResourceId) -> Self {
        Self(id)
    }

    pub fn resource(&self) -> &ResourceId {
        &self.0
    }

    pub fn namespace(&self) -> &str {
        self.0.namespace()
    }

    pub fn path(&self) -> &str {
        self.0.path()
    }

    /// `getSuffix`: `namespace_path`, for names of items and models made from a material.
    pub fn suffix(&self) -> String {
        format!("{}_{}", self.namespace(), self.path())
    }
}

impl fmt::Display for MaterialId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A material, or a material with a variant (`tconstruct:copper#weathered`). `MaterialVariantId.create` returns the plain id for an
/// empty variant.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MaterialVariantId {
    Plain(MaterialId),
    Variant(MaterialId, String),
}

/// `MaterialVariantId.DEFAULT_VARIANT`.
pub const DEFAULT_VARIANT: &str = "default";

impl MaterialVariantId {
    /// `create`: `None` if the variant has a character a path may not.
    pub fn create(id: MaterialId, variant: &str) -> Option<Self> {
        if variant.is_empty() {
            return Some(Self::Plain(id));
        }
        is_valid_path(variant).then(|| Self::Variant(id, variant.to_owned()))
    }

    /// `tryParse(defaultDomain, string)`: `namespace:path#variant`.
    pub fn try_parse_in(default_namespace: &str, text: &str) -> Option<Self> {
        let (name, variant) = match text.split_once('#') {
            Some((name, variant)) => {
                if !is_valid_path(variant) {
                    return None;
                }
                (name, variant)
            }
            None => (text, ""),
        };
        let id = MaterialId::from_resource(ResourceId::try_parse_in(default_namespace, name)?);
        Self::create(id, variant)
    }

    /// `tryParse(string)`: the default namespace is `minecraft`.
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::try_parse_in("minecraft", text)
    }

    pub fn id(&self) -> &MaterialId {
        match self {
            Self::Plain(id) | Self::Variant(id, _) => id,
        }
    }

    /// The variant, empty for none.
    pub fn variant(&self) -> &str {
        match self {
            Self::Plain(_) => "",
            Self::Variant(_, variant) => variant,
        }
    }

    pub fn has_variant(&self) -> bool {
        matches!(self, Self::Variant(..))
    }

    pub fn is_default_variant(&self) -> bool {
        self.variant() == DEFAULT_VARIANT
    }

    /// `normalizeVariant`: `material#default` is just `material`.
    pub fn normalize_variant(&self) -> Self {
        if self.is_default_variant() {
            Self::Plain(self.id().clone())
        } else {
            self.clone()
        }
    }

    pub fn same_id(&self, other: &Self) -> bool {
        self.id() == other.id()
    }

    pub fn same_variant(&self, other: &Self) -> bool {
        self.same_id(other) && self.variant() == other.variant()
    }

    /// `matchesVariant`: a plain id matches every variant of its material, `material#default` matches the material with no variant or
    /// the default one, and any other variant matches only itself.
    pub fn matches_variant(&self, other: &Self) -> bool {
        match self {
            Self::Plain(id) => id == other.id(),
            Self::Variant(id, variant) if variant == DEFAULT_VARIANT => id == other.id() && (other.variant().is_empty() || other.variant() == DEFAULT_VARIANT),
            Self::Variant(..) => self.same_variant(other),
        }
    }

    /// `getLocation(separator)`: the id with `separator` and the variant added to the path; the id itself with no variant.
    pub fn location(&self, separator: char) -> Option<ResourceId> {
        match self {
            Self::Plain(id) => Some(id.resource().clone()),
            Self::Variant(id, variant) => id.resource().with_suffix(&format!("{separator}{variant}")),
        }
    }

    /// `getSuffix`.
    pub fn suffix(&self) -> String {
        match self {
            Self::Plain(id) => id.suffix(),
            Self::Variant(id, variant) => format!("{}_{}", id.suffix(), variant),
        }
    }
}

impl fmt::Display for MaterialVariantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plain(id) => id.fmt(f),
            Self::Variant(id, variant) => write!(f, "{id}#{variant}"),
        }
    }
}

impl From<MaterialId> for MaterialVariantId {
    fn from(id: MaterialId) -> Self {
        Self::Plain(id)
    }
}

/// The id of a kind of material stats (`tconstruct:head`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialStatsId(ResourceId);

impl MaterialStatsId {
    pub fn try_parse(text: &str) -> Option<Self> {
        ResourceId::try_parse(text).map(Self)
    }

    pub fn from_resource(id: ResourceId) -> Self {
        Self(id)
    }

    pub fn resource(&self) -> &ResourceId {
        &self.0
    }
}

impl fmt::Display for MaterialStatsId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> MaterialVariantId {
        MaterialVariantId::try_parse(text).unwrap()
    }

    #[test]
    fn variants_parse_and_print() {
        assert_eq!(v("tconstruct:copper#weathered").to_string(), "tconstruct:copper#weathered");
        assert_eq!(v("tconstruct:copper").to_string(), "tconstruct:copper");
        assert_eq!(v("tconstruct:copper#").to_string(), "tconstruct:copper", "an empty variant is no variant");
        assert!(MaterialVariantId::try_parse("tconstruct:copper#Bad").is_none());
        assert!(MaterialVariantId::try_parse("tconstruct:Copper").is_none());
    }

    #[test]
    fn the_default_variant_normalises_away() {
        assert_eq!(v("tconstruct:copper#default").normalize_variant(), v("tconstruct:copper"));
        assert_eq!(v("tconstruct:copper#weathered").normalize_variant(), v("tconstruct:copper#weathered"));
    }

    #[test]
    fn matching_follows_the_original_rules() {
        let plain = v("tconstruct:copper");
        let weathered = v("tconstruct:copper#weathered");
        let default = v("tconstruct:copper#default");
        assert!(plain.matches_variant(&weathered), "a plain id matches any variant");
        assert!(plain.matches_variant(&plain));
        assert!(!weathered.matches_variant(&plain));
        assert!(weathered.matches_variant(&weathered));
        assert!(default.matches_variant(&plain), "material#default matches no variant");
        assert!(default.matches_variant(&default));
        assert!(!default.matches_variant(&weathered));
        assert!(!plain.matches_variant(&v("tconstruct:iron")));
    }

    #[test]
    fn locations_and_suffixes() {
        assert_eq!(v("tconstruct:copper#weathered").location('_').unwrap().to_string(), "tconstruct:copper_weathered");
        assert_eq!(v("tconstruct:copper").location('_').unwrap().to_string(), "tconstruct:copper");
        assert_eq!(v("tconstruct:copper#weathered").suffix(), "tconstruct_copper_weathered");
        assert_eq!(v("tconstruct:copper").suffix(), "tconstruct_copper");
    }
}
