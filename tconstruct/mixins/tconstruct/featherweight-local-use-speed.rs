// Read the source armor contribution from the existing per-slot movement
// modifiers. The local game synchronizes the active item speed before this
// call. Server prediction has a distinct entry and is intentionally untouched.
fn tconstruct_featherweight_local_use_speed(motion: &mut crate::motion::MotionWorld<'_, '_, '_>) {
    let player = &mut *motion.player;
    if player.is_passenger || player.server_copy || player.replaying { return; }
    let Some(base) = player.item_use_speed else { return; };
    let Some(key) = player.attributes.key("minecraft:movement_speed") else { return; };
    let mut bonus = 0.0_f64;
    for slot in ["head", "chest", "legs", "feet"] {
        let id = format!("tconstruct:featherweight_movement/{slot}");
        if let Ok(amount) = player.attributes.modifier_value(Some(key.clone()), &id) {
            if amount.is_finite() && amount > 0.0 { bonus += amount; }
        }
    }
    if bonus > 0.0 && base.is_finite() {
        player.item_use_speed = Some((f64::from(base) + bonus).clamp(0.0, 1.0) as f32);
    }
}
