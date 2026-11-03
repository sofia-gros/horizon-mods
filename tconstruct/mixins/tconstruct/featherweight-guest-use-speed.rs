// Adjust once when the host initializes guest replay, preserving its sprint flag.
fn tconstruct_featherweight_guest_use_speed(item_use: Option<(f32, bool)>, game: &crate::game::Game<'_>, conn: u32) -> Option<(f32, bool)> {
    let (base, slows) = item_use?;
    let Some((actor, bonus)) = game.world.chunks.__horizon_mixins_tconstruct_featherweight_guest_use_snapshot().as_ref() else { return Some((base, slows)); };
    if *actor != conn || *bonus <= 0.0 || !base.is_finite() { return Some((base, slows)); }
    Some(((f64::from(base) + *bonus).clamp(0.0, 1.0) as f32, slows))
}
