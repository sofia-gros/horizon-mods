fn tconstruct_material_attack_speed(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<f64> {
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    let Some(stack) = game.held().filter(|stack| stack.item.starts_with("tconstruct:")) else { return Continue(()); };
    let Some(Tag::Float(bits)) = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|tag| tag.get("tic_attack_speed")) else { return Continue(()); };
    let speed = game.potions.effects.modifiers().into_iter()
        .filter(|modifier| modifier.0 == "minecraft:attack_speed")
        .fold(f64::from(f32::from_bits(*bits)), |speed, modifier| speed * (1.0 + modifier.2))
        .clamp(0.0, 1024.0);
    Break(speed)
}
