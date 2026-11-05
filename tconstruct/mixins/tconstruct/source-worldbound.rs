// Canonical Worldbound sets IndestructibleItemEntity's volatile flag.
pub(super) const ID: &str = "tconstruct:worldbound";
const DEFINITION: &str = r#"{"level_display":"tconstruct:no_levels","modules":[{"type":"tconstruct:volatile_flag","flag":"tconstruct:indestructible"},{"type":"tconstruct:rarity","rarity":"rare"}]}"#;

pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some_and(|value| Some(value) == serde_json::from_str(DEFINITION).ok())
}

// Unknown damage types retain native behavior. Source Worldbound explicitly
// permits damage tagged as bypassing invulnerability, including pack overrides.
pub(super) fn blocks_lightning(bypasses_invulnerability: Option<bool>) -> bool {
    bypasses_invulnerability == Some(false)
}

// Native expiry grants at most 32766 additional ticks. Repeated extensions
// keep the item alive without the unlimited-age sentinel that stops spinning.
pub(super) fn extend_life(extra_life: i32) -> i32 {
    extra_life.max(32766)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lightning_requires_known_non_bypassing_damage() {
        assert!(blocks_lightning(Some(false)));
        assert!(!blocks_lightning(Some(true)));
        assert!(!blocks_lightning(None));
    }
    #[test]
    fn expiry_preserves_other_positive_extensions() {
        assert_eq!(extend_life(0), 32766);
        assert_eq!(extend_life(-20), 32766);
        assert_eq!(extend_life(40000), 40000);
        assert_ne!(6000 - extend_life(0), i16::MIN as i32);
    }
    #[test]
    fn only_the_canonical_indestructible_flag_is_accepted() {
        assert!(known(Some(DEFINITION.as_bytes())));
        let mut value: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        value["modules"][0]["flag"] = "tconstruct:other".into();
        assert!(!known(Some(&serde_json::to_vec(&value).unwrap())));
        assert!(!known(None));
    }
}
