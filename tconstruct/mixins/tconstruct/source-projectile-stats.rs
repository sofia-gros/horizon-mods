//! Original material projectile multipliers. Injection at firing remains pending.
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

fn stat(stack: &ItemStack, key: &str) -> Option<f32> {
    match stack.data()?.other("minecraft:custom_data")?.get(key)? {
        Tag::Float(bits) => Some(f32::from_bits(*bits)),
        _ => None,
    }
}

pub(crate) fn projectile_stats(stack: &ItemStack) -> Option<(f32, f32)> {
    if !matches!(&*stack.item, "tconstruct:longbow" | "tconstruct:crossbow") { return None; }
    let velocity = stat(stack, "tic_velocity")?;
    let accuracy = stat(stack, "tic_accuracy")?;
    if !velocity.is_finite() || velocity < 0.0 || !accuracy.is_finite() || accuracy <= 0.0 { return None; }
    Some((velocity, 3.0 * (1.0 / accuracy - 1.0)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn source_formula() {
        let (velocity, accuracy) = (1.25_f32, 0.75_f32);
        let spread = 3.0 * (1.0 / accuracy - 1.0);
        assert!((velocity * 3.0 - 3.75).abs() < 1e-6);
        assert!((spread - 1.0).abs() < 1e-6);
    }
}
