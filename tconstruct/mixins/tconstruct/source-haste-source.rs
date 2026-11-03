// Exact pinned MIT source definition. A pack override, even one differing only
// in whitespace, remains unknown until full modifier loading is implemented.
pub(super) const PINNED_HASTE: &[u8] = br#"{
  "level_display": {
    "type": "tconstruct:unique",
    "first_unique": false,
    "unique_until": 5
  },
  "modules": [
    {
      "type": "tconstruct:stat_boost",
      "each_level": 4.0,
      "operation": "add",
      "stat": "tconstruct:mining_speed"
    },
    {
      "type": "tconstruct:attribute",
      "attribute": "tconstruct:player.mining_speed_multiplier",
      "each_level": 0.1,
      "operation": "multiply_total",
      "slots": [
        "mainhand",
        "offhand",
        "feet",
        "legs",
        "chest",
        "head"
      ],
      "tool": {
        "type": "mantle:inverted",
        "inverted_type": "mantle:tag",
        "tag": "tconstruct:modifiable/harvest"
      }
    }
  ]
}"#;

/// None is deliberately distinct from an empty/negative harvest tag.
pub(super) fn source_haste_trait_allowed(is_harvest: Option<bool>, source: Option<&[u8]>) -> bool {
    is_harvest == Some(false) && source == Some(PINNED_HASTE)
}


#[cfg(test)] mod tests {
    use super::*;
    #[test]
    fn trait_requires_local_actor_fresh_tag_and_exact_source() {
        assert!(source_haste_trait_allowed(Some(false), Some(PINNED_HASTE)));
        assert!(!source_haste_trait_allowed(None, Some(PINNED_HASTE)));
        assert!(!source_haste_trait_allowed(Some(true), Some(PINNED_HASTE)));
        assert!(!source_haste_trait_allowed(Some(false), None));
        assert!(!source_haste_trait_allowed(Some(false), Some(b"{}")));
    }
}
