// PartCastItem registrations in TinkerSmeltery; the three cast variants
// share a supplier for the corresponding material item and its recipe cost.
pub(super) const PARTS: [&str; 20] = [
    "repair_kit", "pick_head", "small_axe_head", "small_blade", "adze_head",
    "hammer_head", "broad_axe_head", "broad_blade", "large_plate", "tool_binding",
    "tough_binding", "tool_handle", "tough_handle", "bow_limb", "bow_grip",
    "helmet_plating", "chestplate_plating", "leggings_plating", "boots_plating", "maille",
];

pub(super) fn cast_index(item: &str) -> Option<usize> {
    let short = item.strip_prefix("tconstruct:")?;
    let base = short.strip_suffix("_red_sand_cast")
        .or_else(|| short.strip_suffix("_sand_cast"))
        .or_else(|| short.strip_suffix("_cast"))?;
    PARTS.iter().position(|part| *part == base)
}

pub(super) fn parse_costs(values: &[String]) -> Option<Vec<i32>> {
    if values.len() != PARTS.len() { return None; }
    values.iter().map(|s| s.parse::<i32>().ok().filter(|n| *n >= 0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_part_cast_mapping_and_complete_frame() {
        let i = cast_index("tconstruct:pick_head_cast").unwrap();
        assert_eq!(cast_index("tconstruct:pick_head_sand_cast"), Some(i));
        assert_eq!(cast_index("tconstruct:pick_head_red_sand_cast"), Some(i));
        assert_eq!(cast_index("tconstruct:ingot_cast"), None);
        let mut values = vec!["0".to_owned(); PARTS.len()];
        values[i] = "2147483647".to_owned();
        assert_eq!(parse_costs(&values).unwrap()[i], i32::MAX);
        assert!(parse_costs(&values[..19]).is_none());
        values[i] = "-1".to_owned();
        assert!(parse_costs(&values).is_none());
    }
}
