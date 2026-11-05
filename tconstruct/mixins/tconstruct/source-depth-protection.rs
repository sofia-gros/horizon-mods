pub(crate) fn supported_value(value: &serde_json::Value) -> bool {
    value == &serde_json::json!({"level_display":"tconstruct:default","modules":[{
        "type":"tconstruct:depth_protection","baseline_height":64.0,
        "damage_source":"mantle:can_protect","each_level":1.25,"neutral_range":32.0
    }]})
}

/// Source casts the entity Y to float before the height arithmetic. Do not
/// floor it to a block coordinate or use double intermediates.
pub(crate) fn points(y: f64, levels: &[i32]) -> Option<f32> {
    let y = y as f32;
    if !y.is_finite() { return None; }
    let factor = if y < 64.0 {
        ((64.0 - y) / 64.0).min(2.0)
    } else if y > 96.0 {
        ((96.0 - y) / 64.0).max(-1.0)
    } else { 0.0 };
    let mut points = 0.0f32;
    for level in levels {
        if *level < 0 { return None; }
        points += factor * (1.25 * *level as f32);
    }
    points.is_finite().then_some(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_height_bands_include_the_high_altitude_penalty() {
        for (y,expected) in [(-128.0,2.5),(-64.0,2.5),(0.0,1.25),(32.0,0.625),(64.0,0.0),(80.0,0.0),(96.0,0.0),(128.0,-0.625),(160.0,-1.25),(300.0,-1.25)] {
            assert_eq!(points(y,&[1]),Some(expected));
        }
    }
    #[test]
    fn pieces_add_and_height_is_not_rounded_to_a_block() {
        assert_eq!(points(0.0,&[1,2]),Some(3.75));
        assert_eq!(points(63.5,&[1]),Some((0.5 / 64.0) * 1.25));
        assert_eq!(points(128.0,&[]),Some(0.0));
        assert_eq!(points(f64::NAN,&[1]),None);
        assert_eq!(points(0.0,&[-1]),None);
    }
    #[test]
    fn changed_definition_is_not_assumed_to_have_the_stock_formula() {
        let mut value = serde_json::json!({"level_display":"tconstruct:default","modules":[{
            "type":"tconstruct:depth_protection","baseline_height":64.0,
            "damage_source":"mantle:can_protect","each_level":1.25,"neutral_range":32.0
        }]});
        assert!(supported_value(&value));
        value["modules"][0]["neutral_range"] = serde_json::json!(0.0);
        assert!(!supported_value(&value));
    }
}
