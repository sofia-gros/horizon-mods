pub(crate) fn supported_value(value: &serde_json::Value) -> bool {
    let module = |first:&str,second:&str,water:&str|serde_json::json!({
        "type":"tconstruct:protection","damage_source":"mantle:can_protect","each_level":2.5,
        "tool":{"type":"mantle:or","predicates":[
            {"type":"mantle:tag","tag":first},{"type":"mantle:tag","tag":second}
        ]},"wearing_entity":water
    });
    value == &serde_json::json!({"level_display":"tconstruct:default","modules":[
        {"type":"tconstruct:max_armor_attribute","allow_broken":false,"attribute":"forge:swim_speed",
         "each_level":0.1,"held_tag":"tconstruct:modifiable/held","operation":"multiply_total"},
        module("tconstruct:modifiable/armor/helmets","tconstruct:modifiable/armor/chestplate","mantle:eyes_in_water"),
        module("tconstruct:modifiable/armor/leggings","tconstruct:modifiable/armor/boots","mantle:feet_in_water")
    ]})
}

/// Each tool condition is independent. Custom tags may make both modules
/// match one item; neither the equipped slot nor a single underwater flag
/// replaces those conditions.
pub(crate) fn points(levels: &[(f32,bool,bool)], eyes: bool, feet: bool) -> Option<f32> {
    let mut points = 0.0f32;
    for (level,upper,lower) in levels {
        if !level.is_finite() || *level < 0.0 { return None; }
        if *upper && eyes { points += 2.5 * *level; }
        if *lower && feet { points += 2.5 * *level; }
    }
    points.is_finite().then_some(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upper_and_lower_pieces_use_distinct_water_predicates() {
        let levels = [(1.0,true,false),(2.0,false,true)];
        assert_eq!(points(&levels,false,false),Some(0.0));
        assert_eq!(points(&levels,true,false),Some(2.5));
        assert_eq!(points(&levels,false,true),Some(5.0));
        assert_eq!(points(&levels,true,true),Some(7.5));
    }
    #[test]
    fn fractional_levels_and_overlapping_tool_tags_are_preserved() {
        assert_eq!(points(&[(0.4,true,false)],true,true),Some(2.5 * 0.4));
        assert_eq!(points(&[(1.0,true,true)],true,true),Some(5.0));
        assert_eq!(points(&[(2.0,false,false)],true,true),Some(0.0));
    }
    #[test]
    fn invalid_levels_are_rejected_and_absent_levels_are_neutral() {
        assert_eq!(points(&[(f32::NAN,true,false)],true,true),None);
        assert_eq!(points(&[(-1.0,true,false)],true,true),None);
        assert_eq!(points(&[],true,true),Some(0.0));
    }
}
