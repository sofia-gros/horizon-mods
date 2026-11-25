// Read-only potion/effect catalog facts. Content and application policy stay in consumers.
use sunrise_client_core::potions;

/// A registered effect without exposing the host's definition type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectFacts {
    /// Native storage identity: vanilla is short, custom content is namespaced.
    pub raw_id: &'static str,
    /// Fully qualified native mob-storage identity, preserving custom namespaces.
    pub canonical_id: &'static str,
    /// Unmixed RGB. The caller owns alpha, packing and averaging rules.
    pub color_rgb: u32,
    /// Native catalog flag, not permission to apply this effect.
    pub instant: bool,
    /// Whether the effect's category is beneficial (a harmful or neutral effect is not).
    pub beneficial: bool,
}

/// Return facts for the host's existing registered effect, or None if unknown.
/// Lookup keeps the official host's alias behavior; it does not validate IDs,
/// register content, inspect active effects or apply any effect.
pub fn effect(id: &str) -> Option<EffectFacts> {
    let def = potions::effect(id)?;
    Some(EffectFacts {
        raw_id: def.id,
        canonical_id: potions::namespaced(def.id),
        color_rgb: def.color,
        instant: def.instant,
        beneficial: def.category == potions::Category::Beneficial,
    })
}

/// A registered potion without exposing the host's definition type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PotionFacts {
    pub raw_id: &'static str,
    /// The native translation suffix, not a localized name or translation key.
    pub name_suffix: &'static str,
    /// Ordered (raw effect ID, duration ticks, amplifier) values, unchanged.
    /// Empty potions remain known; this slice is not filtered or deduplicated.
    pub effects: &'static [(&'static str, i32, i32)],
}

/// Query the existing native potion catalog, retaining empty and unknown as
/// distinct outcomes. No cache, source-version allowlist or effect cap is added.
pub fn potion(id: &str) -> Option<PotionFacts> {
    let def = potions::potion(id)?;
    Some(PotionFacts { raw_id: def.id, name_suffix: def.name, effects: def.effects })
}

/// Match the host's syntax-only custom-namespace classification. This does not
/// prove that an ID is valid or registered; even an unknown namespaced ID can
/// return true. Use effect() separately when catalog membership is required.
pub fn has_custom_namespace(id: &str) -> bool {
    potions::is_mod_effect(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_category_read_is_beneficial_for_speed_and_not_for_poison() {
        assert_eq!(effect("minecraft:speed").map(|facts| facts.beneficial), Some(true), "speed is beneficial");
        assert_eq!(effect("minecraft:poison").map(|facts| facts.beneficial), Some(false), "poison is harmful");
        assert!(effect("minecraft:unknown_effect_xyz").is_none());
    }
}

