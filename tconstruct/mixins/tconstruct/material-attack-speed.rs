fn tconstruct_material_attack_speed(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<f64> {
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    let Some(stack) = game.held().filter(|stack| stack.item.starts_with("tconstruct:")) else { return Continue(()); };
    let Some(Tag::Float(bits)) = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|tag| tag.get("tic_attack_speed")) else { return Continue(()); };
    // The source tool stat includes the default base of four; its held
    // attribute modifier is stat - 4, added to the player's current base.
    // Match the host lookup so /attribute base set still applies.
    let base = game.player.as_ref().and_then(|player| {
        let key = player.attributes.key("minecraft:attack_speed")?;
        let handle = player.attributes.lookup(Some(&key))?;
        player.attributes.instance(&handle).ok().map(|value| value.base_value())
    }).unwrap_or(horizon_client::attack_strength::BASE_ATTACK_SPEED);
    let value = base + (f64::from(f32::from_bits(*bits))
        - horizon_client::attack_strength::BASE_ATTACK_SPEED);
    let speed = game.potions.effects.modifiers().into_iter()
        .filter(|modifier| modifier.0 == "minecraft:attack_speed")
        .fold(value, |speed, modifier| speed * (1.0 + modifier.2))
        .clamp(0.0, 1024.0);
    Break(speed)
}
