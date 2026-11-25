// Borrowed resident damage facts. No registry, cache, derived policy or entity
// lookup lives here. All classification delegates to official DamageSource.
use super::{Catalog, DamageSource, DamageTypeId, Definition};
use crate::{damage_immunity::ImmunityTag, damage_reduction::Bypass, living_hurt::HurtTag};

#[derive(Clone, Copy, Debug)]
pub struct SourceView<'a> {
    id: DamageTypeId,
    source: Option<DamageSource<'a, ()>>,
}
impl<'a> SourceView<'a> {
    /// Use the same catalog/id pair that the native hit will receive. IDs are
    /// catalog-local, not wire IDs or persistent/global identity.
    /// Unknown IDs stay unknown; this does not decide whether a hit succeeds.
    pub fn new(catalog: &'a Catalog, id: DamageTypeId) -> Self {
        Self {
            id,
            source: catalog.source(id, None, None, None),
        }
    }
    pub fn id(&self) -> DamageTypeId {
        self.id
    }
    pub fn is_known(&self) -> bool {
        self.source.is_some()
    }
    pub fn definition(&self) -> Option<&Definition> {
        self.source.as_ref().map(DamageSource::definition)
    }
    pub fn name(&self) -> Option<&str> {
        self.definition().map(|d| d.name.as_str())
    }
    pub fn is_type(&self, name: &str) -> Option<bool> {
        self.source.as_ref().map(|source| source.is_type(name))
    }
    /// None: invalid source ID. Some(false): known source absent from that tag,
    /// including a missing tag, exactly as the native resident catalog answers.
    pub fn in_tag(&self, tag: &str) -> Option<bool> {
        self.source.as_ref().map(|source| source.in_tag(tag))
    }
    pub fn bypasses(&self, kind: Bypass) -> Option<bool> {
        self.source.as_ref().map(|source| source.bypasses(kind))
    }
    pub fn hurt_tag(&self, tag: HurtTag) -> Option<bool> {
        self.source.as_ref().map(|source| source.hurt_tag(tag))
    }
    pub fn immunity_tag(&self, tag: ImmunityTag) -> Option<bool> {
        self.source.as_ref().map(|source| source.immunity_tag(tag))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn catalog(names: &[&str], fire: bool) -> Catalog {
        let types: Vec<_> = names
            .iter()
            .map(|name| {
                json!({
                    "id": name, "message_id": "test", "exhaustion": "3dcccccd",
                    "scaling": "never", "effects": "hurt", "death_message_type": "default"
                })
            })
            .collect();
        let mut tags = json!({});
        if fire {
            for tag in ["minecraft:is_fire", "minecraft:bypasses_armor"] {
                tags[tag] = json!({"values":[names[0]]}).to_string().into();
            }
        }
        Catalog::from_json(
            &json!({"data_version":crate::DATA_VERSION,"types":types}),
            &tags,
        )
        .unwrap()
    }
    #[test]
    fn pluto_source_context_matches_native_queries() {
        let catalog = catalog(&["example:heat"], true);
        let id = catalog.id("example:heat").unwrap();
        let view = SourceView::new(&catalog, id);
        let native = catalog.source::<()>(id, None, None, None).unwrap();
        assert!(view.is_known());
        assert_eq!(view.id(), id);
        assert_eq!(view.name(), Some(native.definition().name.as_str()));
        assert_eq!(
            view.definition().unwrap().exhaustion.to_bits(),
            native.food_exhaustion().to_bits()
        );
        assert_eq!(view.is_type("example:heat"), Some(true));
        for tag in [
            "minecraft:is_fire",
            "minecraft:bypasses_armor",
            "example:missing",
        ] {
            assert_eq!(view.in_tag(tag), Some(native.in_tag(tag)));
        }
        for kind in [
            Bypass::Armor,
            Bypass::Effects,
            Bypass::Resistance,
            Bypass::Enchantments,
        ] {
            assert_eq!(view.bypasses(kind), Some(native.bypasses(kind)));
        }
        for tag in [
            HurtTag::Fire,
            HurtTag::Freezing,
            HurtTag::DamagesHelmet,
            HurtTag::BypassesCooldown,
            HurtTag::NoImpact,
            HurtTag::NoKnockback,
        ] {
            assert_eq!(view.hurt_tag(tag), Some(native.hurt_tag(tag)));
        }
        for tag in [
            ImmunityTag::BypassesInvulnerability,
            ImmunityTag::Fire,
            ImmunityTag::Fall,
            ImmunityTag::Drowning,
            ImmunityTag::Freezing,
        ] {
            assert_eq!(view.immunity_tag(tag), Some(native.immunity_tag(tag)));
        }
    }
    #[test]
    fn pluto_source_context_unknown_id_is_not_a_negative_classification() {
        let larger = catalog(&["example:first", "example:second"], false);
        let smaller = catalog(&["example:first"], true);
        let id = larger.id("example:second").unwrap();
        assert!(smaller.source::<()>(id, None, None, None).is_none());
        let view = SourceView::new(&smaller, id);
        assert!(!view.is_known());
        assert!(view.definition().is_none());
        assert_eq!(view.name(), None);
        assert_eq!(view.is_type("example:second"), None);
        assert_eq!(view.in_tag("minecraft:is_fire"), None);
        assert_eq!(view.bypasses(Bypass::Armor), None);
        assert_eq!(view.hurt_tag(HurtTag::Fire), None);
        assert_eq!(view.immunity_tag(ImmunityTag::Fire), None);
    }
    #[test]
    fn pluto_source_context_catalogs_do_not_share_cached_names_or_tags() {
        let first = catalog(&["example:heat"], true);
        let second = catalog(&["example:cold"], false);
        let a = first.id("example:heat").unwrap();
        let b = second.id("example:cold").unwrap();
        assert_eq!(a, b, "native local index may repeat in another catalog");
        let first_view = SourceView::new(&first, a);
        let second_view = SourceView::new(&second, b);
        assert_eq!(first_view.name(), Some("example:heat"));
        assert_eq!(second_view.name(), Some("example:cold"));
        assert_eq!(first_view.in_tag("minecraft:is_fire"), Some(true));
        assert_eq!(second_view.in_tag("minecraft:is_fire"), Some(false));
    }
    #[test]
    fn pluto_source_context_absent_tags_are_false_for_known_sources() {
        let catalog = catalog(&["example:plain"], false);
        let view = SourceView::new(&catalog, catalog.id("example:plain").unwrap());
        assert_eq!(view.in_tag("example:unknown"), Some(false));
        assert_eq!(view.is_type("example:other"), Some(false));
        assert_eq!(view.bypasses(Bypass::Armor), Some(false));
    }
}
