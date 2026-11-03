// Source ToolPartItem advanced material ID, restricted to the built-in part
// registrations. It is derived only from the stack, so no remote registry is
// guessed. The unknown sentinel and malformed IDs do not produce a line.
pub(super) fn source_part_material_id(item: &str, material: &str, advanced: bool, hidden: bool, display: bool) -> Option<String> {
    if !advanced || hidden || display { return None; }
    let part = item.strip_prefix("tconstruct:")?;
    if !["pick_head", "hammer_head", "small_axe_head", "broad_axe_head", "small_blade", "broad_blade", "adze_head", "large_plate", "bow_limb", "bow_grip", "bowstring", "arrow_head", "arrow_shaft", "fletching", "tool_binding", "tough_binding", "tool_handle", "tough_handle", "helmet_plating", "chestplate_plating", "leggings_plating", "boots_plating", "maille", "shield_core", "ribcage", "shell", "laces"].contains(&part) { return None; }
    if material.is_empty() { return None; }
    let (base, variant) = material.split_once('#').unwrap_or((material, ""));
    let (namespace, path) = base.split_once(':').unwrap_or(("minecraft", base));
    let namespace = if namespace.is_empty() { "minecraft" } else { namespace };
    if path.is_empty() || !namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
        || !path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
        || !variant.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c)) { return None; }
    let base = format!("{namespace}:{path}");
    if variant.is_empty() {
        if base == "tconstruct:unknown" { return None; }
        Some(base)
    } else { Some(format!("{base}#{variant}")) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn advanced_material_id_is_scoped_and_canonical() {
        for part in ["pick_head", "hammer_head", "small_axe_head", "broad_axe_head", "small_blade", "broad_blade", "adze_head", "large_plate", "bow_limb", "bow_grip", "bowstring", "arrow_head", "arrow_shaft", "fletching", "tool_binding", "tough_binding", "tool_handle", "tough_handle", "helmet_plating", "chestplate_plating", "leggings_plating", "boots_plating", "maille", "shield_core", "ribcage", "shell", "laces"] {
            assert_eq!(source_part_material_id(&format!("tconstruct:{part}"), "tconstruct:wood#spruce", true, false, false).as_deref(), Some("tconstruct:wood#spruce"));
        }
        for item in ["tconstruct:repair_kit", "tconstruct:fake_ingot", "tconstruct:pickaxe", "minecraft:pick_head"] {
            assert_eq!(source_part_material_id(item, "tconstruct:wood", true, false, false), None);
        }
        assert_eq!(source_part_material_id("tconstruct:pick_head", "wood", true, false, false).as_deref(), Some("minecraft:wood"));
        assert_eq!(source_part_material_id("tconstruct:pick_head", ":wood#", true, false, false).as_deref(), Some("minecraft:wood"));
        for bad in ["", "tconstruct:unknown", "Bad:wood", "tconstruct:", "tconstruct:wood#Bad", "tconstruct:wood#one#two", "tconstruct:wood:bad", "tconstruct:wood#bad:variant"] {
            assert_eq!(source_part_material_id("tconstruct:pick_head", bad, true, false, false), None);
        }
    }
    #[test]
    fn flags_suppress_details() {
        for (advanced, hidden, display) in [(false,false,false),(true,true,false),(true,false,true)] {
            assert_eq!(source_part_material_id("tconstruct:pick_head", "tconstruct:wood", advanced, hidden, display), None);
        }
    }
}
