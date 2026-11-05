// Pinned Padded reduces knockback dealt, not the holder's incoming knockback.
pub(super) const ID: &str = "tconstruct:padded";
pub(super) fn known(bytes: Option<&[u8]>) -> bool {
    bytes.and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok()).is_some_and(|v| v == serde_json::json!({
        "level_display":"tconstruct:default", "priority":75, "modules":[
            {"type":"tconstruct:knockback","formula":["$knockback",2.0,"$level","^","/"]},
            {"type":"tconstruct:sling_force","formula":["$value",2.0,"$level","^","/"],"percent":false,"sling":{"type":"tconstruct:tag","tag":"tconstruct:knockback_slings"}}
        ]
    }))
}
pub(super) fn reduce(base: f32, level: i32) -> Option<f32> {
    if !base.is_finite() || base < 0.0 || level <= 0 {
        return None;
    }
    // The source formula's power result is a float; 2^128 overflows to infinity.
    Some(if level >= 128 {
        0.0
    } else {
        base / 2.0_f32.powi(level)
    })
}
pub(super) fn split(total: f32) -> (bool, f32) {
    if total < 0.4_f32 {
        (true, total)
    } else {
        (false, total - 0.4_f32)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinary_and_sprinting_hits_scale_the_whole_knockback() {
        assert_eq!(reduce(0.4, 1), Some(0.2));
        let total = reduce(0.4 + 0.5, 1).unwrap();
        assert_eq!(total, 0.45);
        assert_eq!(split(total), (false, 0.45_f32 - 0.4_f32));
        assert_eq!(split(reduce(0.9, 2).unwrap()), (true, 0.225));
    }
    #[test]
    fn boundary_zero_and_invalid_levels_do_not_wrap() {
        assert_eq!(split(0.4), (false, 0.0));
        assert_eq!(reduce(1.0, 128), Some(0.0));
        assert_eq!(reduce(1.0, i32::MAX), Some(0.0));
        assert_eq!(reduce(0.4, 0), None);
        assert_eq!(reduce(f32::NAN, 1), None);
        assert_eq!(reduce(-1.0, 1), None);
    }
}
