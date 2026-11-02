fn tconstruct_material_melee_bonus(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<f32> {
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    let Some(stack) = game
        .held()
        .filter(|stack| stack.item.starts_with("tconstruct:"))
    else {
        return Continue(());
    };
    if game.attack_item.as_deref() != Some(&*stack.item) {
        return Continue(());
    }
    let Some(Tag::Float(bits)) = stack
        .data()
        .and_then(|d| d.other("minecraft:custom_data"))
        .and_then(|tag| tag.get("tic_attack_damage"))
    else {
        return Continue(());
    };
    let native = game
        .attack_item_info()
        .map_or(0.0, |item| item.attack_damage);
    let damage = f32::from_bits(*bits);
    let trait_bonus = crate::game::tconstruct_source_material_traits::levels(&game.world, stack)
        .map(|traits| {
            let jagged = traits.levels.get("tconstruct:jagged").copied().unwrap_or(0);
            let stonebound = traits
                .levels
                .get("tconstruct:stonebound")
                .copied()
                .unwrap_or(0);
            if jagged == 0 && stonebound == 0 {
                return 0.0;
            }
            crate::game::tconstruct_source_material_effects::wear_damage_bonus(
                1.0 + damage,
                stack.damage,
                traits.durability_multiplier,
                jagged,
                stonebound,
            )
        })
        .unwrap_or(0.0);
    // Evaluate an additive replacement on a clone of the player's attribute
    // map. The host applies strength, weakness, and multipliers in that map;
    // changing the returned number afterward would bypass their ordering.
    let Some(mut projected) = game.player.clone() else { return Continue(()); };
    if projected.set_modifier("minecraft:attack_damage", "tconstruct:material_attack_projection", f64::from(damage + trait_bonus - native), "add_value", true).is_err() {
        return Continue(());
    }
    match projected.attribute("minecraft:attack_damage") {
        Ok(value) => Break(value as f32),
        Err(_) => Continue(()),
    }
}
