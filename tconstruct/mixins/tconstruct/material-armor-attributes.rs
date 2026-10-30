fn tconstruct_material_armor(game: &mut crate::game::Game<'_>) {
    use horizon_world::nbt::Tag;
    let mut values = Vec::new();
    for (stack, slot) in game.armor.iter().zip(["head", "chest", "legs", "feet"]) {
        let Some(stack) = stack.as_ref().filter(|stack| stack.item.starts_with("tconstruct:")) else { continue; };
        let Some(data) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { continue; };
        for (key, attribute) in [("tic_armor", "minecraft:armor"), ("tic_armor_toughness", "minecraft:armor_toughness"), ("tic_knockback_resistance", "minecraft:knockback_resistance")] {
            if let Some(Tag::Float(bits)) = data.get(key) {
                values.push((attribute.to_owned(), format!("tconstruct:{key}/{slot}"), f64::from(f32::from_bits(*bits))));
            }
        }
    }
    let Some(player) = game.player.as_mut() else { return; };
    for (attribute, id, amount) in values {
        if player.set_modifier(&attribute, &id, amount, "add_value", true).is_ok() {
            game.applied_modifiers.push((attribute, id));
        }
    }
}
