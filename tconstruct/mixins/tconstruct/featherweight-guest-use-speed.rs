// Adjust once when the host initializes the guest's replay speed, not once
// per replayed tick. The following host assignment retains the sprint flag.
fn tconstruct_featherweight_guest_use_speed(item_use: Option<(f32, bool)>, motion: &&mut PlayerMotion) -> Option<(f32, bool)> {
    let (base, slows) = item_use?;
    if motion.is_passenger || !base.is_finite() { return Some((base, slows)); }
    let mut bonus = 0.0_f64;
    if let Some((_, _, instance)) = motion.attributes.instances().find(|(_, key, _)| key.name() == "minecraft:movement_speed") {
        for slot in ["head", "chest", "legs", "feet"] {
            let id = format!("tconstruct:featherweight_movement/{slot}");
            if let Some(modifier) = instance.modifier(&id) {
                let amount = modifier.amount();
                if amount.is_finite() && amount > 0.0 { bonus += amount; }
            }
        }
    }
    Some(((f64::from(base) + bonus).clamp(0.0, 1.0) as f32, slows))
}
