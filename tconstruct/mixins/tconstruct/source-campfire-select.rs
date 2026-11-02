/// The source pathing action can extinguish either lit vanilla campfire.
pub(super) fn is_lit_campfire(name: &str, lit: Option<&str>) -> bool {
    matches!(name, "minecraft:campfire" | "minecraft:soul_campfire") && lit == Some("true")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_lit_campfires_are_selected() {
        for name in ["minecraft:campfire", "minecraft:soul_campfire"] {
            assert!(is_lit_campfire(name, Some("true")));
            assert!(!is_lit_campfire(name, Some("false")));
            assert!(!is_lit_campfire(name, None));
        }
        assert!(!is_lit_campfire("minecraft:furnace", Some("true")));
    }
}
