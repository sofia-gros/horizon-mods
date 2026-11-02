//! Source right-click selection and nearby entity geometry for splashing.
//! Runtime effect handling and the Station recipe remain disabled until the
//! authoritative target enumeration includes guest players and other entities.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

/// DualOptionInteraction routes splashing to right click unless its modifier
/// appears in the source persistent attack_modifiers set. Only tools whose
/// original definition explicitly uses that interaction are considered here.
pub(super) fn right_click(stack: &ItemStack) -> bool {
    if !matches!(&*stack.item, "tconstruct:earth_staff" | "tconstruct:ender_staff" | "tconstruct:ichor_staff" | "tconstruct:sky_staff" | "tconstruct:melting_pan") { return false; }
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return false; };
    if !matches!(custom.get("tic_upgrades").and_then(|upgrades| upgrades.get("tconstruct:splashing")), Some(Tag::Int(level)) if *level > 0) { return false; }
    match custom.get("tic_persistent").and_then(|data| data.get("tconstruct:attack_modifiers")) {
        None => true,
        Some(Tag::List(entries)) => !entries.iter().any(|entry| matches!(entry, Tag::String(id) if id.to_string_lossy() == "tconstruct:splashing")),
        _ => false,
    }
}

/// The source tests the candidate box against the target's inflated box and
/// then compares full 3D entity positions using a strict squared radius.
pub(super) fn in_range(target: [f64; 3], candidate: [f64; 3], target_bounds: ([f64; 3], [f64; 3]), candidate_bounds: ([f64; 3], [f64; 3]), expanded: u32) -> bool {
    let radius = 1.0 + f64::from(expanded);
    let inflation = [radius, 0.25, radius];
    let intersects = (0..3).all(|i| target_bounds.0[i] - inflation[i] < candidate_bounds.1[i] && target_bounds.1[i] + inflation[i] > candidate_bounds.0[i]);
    intersects && (0..3).map(|i| { let delta = target[i] - candidate[i]; delta * delta }).sum::<f64>() < radius * radius
}
