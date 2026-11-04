use horizon_world::nbt::Tag;
pub(super) fn trait_level(custom: &Tag) -> i32 {
    let Some(Tag::List(traits)) = custom.get("tic_mining_traits") else { return 0; };
    traits.iter().find_map(|entry| match (entry.get("id"), entry.get("level")) {
        (Some(Tag::String(id)), Some(Tag::Int(level))) if id.to_string_lossy() == "tconstruct:momentum" => Some((*level).max(0)),
        _ => None,
    }).unwrap_or(0)
}
// Pure source Momentum rules; effect lifetime and hidden upgrades use the host's
// MobEffectInstance implementation rather than item-local data.
pub(super) fn harvest_duration(hardness: f32, mining_speed: f32) -> i32 {
    ((50.0f32 * hardness / mining_speed) as i32).max(100)
}
pub(super) fn factor(effect_level: i32, modifier_level: i32) -> f32 {
    1.0 + (effect_level.max(0) as f32 * modifier_level.max(0) as f32) / 40.0
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn trait_level_requires_the_exact_modifier() {
        let custom = Tag::compound([("tic_mining_traits", Tag::List(vec![
            Tag::compound([("id", Tag::String("tconstruct:momentum".into())), ("level", Tag::Int(2))]),
        ]))]);
        assert_eq!(trait_level(&custom), 2);
        assert_eq!(trait_level(&Tag::compound(Vec::<(&str, Tag)>::new())), 0);
    }
    #[test] fn source_duration_and_multiplier() {
        assert_eq!(harvest_duration(1.0, 4.0), 100);
        assert_eq!(harvest_duration(40.0, 4.0), 500);
        assert_eq!(harvest_duration(1.0, 0.0), i32::MAX);
        assert!((factor(10, 2)-1.5).abs() < f32::EPSILON);
        assert_eq!(factor(0, 1), 1.0);
    }
}
