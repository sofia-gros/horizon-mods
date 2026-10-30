fn tconstruct_material_melee_bonus(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<f32> {
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    let Some(stack) = game.held().filter(|stack| stack.item.starts_with("tconstruct:")) else { return Continue(()); };
    if game.attack_item.as_deref() != Some(&*stack.item) { return Continue(()); }
    let Some(Tag::Float(bits)) = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|tag| tag.get("tic_attack_damage")) else { return Continue(()); };
    let effects = &game.potions.effects;
    let bonus = effects.amplifier("strength").map_or(0.0, |a| 3.0 * (a + 1) as f32)
        - effects.amplifier("weakness").map_or(0.0, |a| 4.0 * (a + 1) as f32);
    let native = game.attack_item_info().map_or(0.0, |item| item.attack_damage);
    Break(bonus + f32::from_bits(*bits) - native)
}
