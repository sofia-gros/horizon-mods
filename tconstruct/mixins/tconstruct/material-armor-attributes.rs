fn tconstruct_material_armor(game: &mut crate::game::Game<'_>) {
    use horizon_world::nbt::Tag;
    let mut values = Vec::new();
    for (stack, slot) in game.armor.iter().zip(["head", "chest", "legs", "feet"]) {
        let Some(stack) = stack.as_ref().filter(|stack| stack.item.starts_with("tconstruct:")) else { continue; };
        if crate::game::tconstruct_source_ranged_wear::is_broken(stack) { continue; }
        let Some(data) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { continue; };
        for (key, attribute) in [("tic_armor", "minecraft:armor"), ("tic_armor_toughness", "minecraft:armor_toughness"), ("tic_knockback_resistance", "minecraft:knockback_resistance")] {
            if let Some(Tag::Float(bits)) = data.get(key) {
                values.push((attribute.to_owned(), format!("tconstruct:{key}/{slot}"), f64::from(f32::from_bits(*bits)), "add_value"));
            }
        }
        let trait_level = crate::game::tconstruct_source_material_traits::levels(&game.world, stack)
            .and_then(|traits| traits.levels.get("tconstruct:vintage").copied()).unwrap_or(0);
        let upgrade_level = match data.get("tic_upgrades").and_then(|upgrades| upgrades.get("tconstruct:vintage")) {
            Some(Tag::Int(level)) => (*level).max(0), _ => 0,
        };
        let level = trait_level.max(0) + upgrade_level;
        if level > 0 {
            values.push(("minecraft:movement_speed".to_owned(), format!("tconstruct:vintage_movement/{slot}"), -0.1 * f64::from(level), "add_multiplied_total"));
            values.push(("minecraft:gravity".to_owned(), format!("tconstruct:vintage_gravity/{slot}"), 0.1 * f64::from(level), "add_multiplied_total"));
        }
    }
    let Some(player) = game.player.as_mut() else { return; };
    for (attribute, id, amount, op) in values {
        if player.set_modifier(&attribute, &id, amount, op, true).is_ok() {
            game.applied_modifiers.push((attribute, id));
        }
    }
}
