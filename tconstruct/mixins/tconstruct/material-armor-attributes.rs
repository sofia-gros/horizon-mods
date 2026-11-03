fn tconstruct_material_armor(game: &mut crate::game::Game<'_>) {
    use horizon_world::nbt::Tag;
    let mut values = Vec::new();
    for (stack, slot) in game.armor.iter().zip(["head", "chest", "legs", "feet"]) {
        let Some(stack) = stack.as_ref().filter(|stack| stack.item.starts_with("tconstruct:")) else { continue; };
        let broken = crate::game::tconstruct_source_ranged_wear::is_broken(stack);
        let Some(data) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { continue; };
        if !broken {
            for (key, attribute) in [("tic_armor", "minecraft:armor"), ("tic_armor_toughness", "minecraft:armor_toughness"), ("tic_knockback_resistance", "minecraft:knockback_resistance")] {
                if let Some(Tag::Float(bits)) = data.get(key) {
                    values.push((attribute.to_owned(), format!("tconstruct:{key}/{slot}"), f64::from(f32::from_bits(*bits)), "add_value"));
                }
            }
        }
        let traits = crate::game::tconstruct_source_material_traits::levels(&game.world, stack);
        let modifier_level = |id: &str| {
            let trait_level = traits.as_ref().and_then(|traits| traits.levels.get(id).copied()).unwrap_or(0).max(0);
            let upgrade_level = match data.get("tic_upgrades").and_then(|upgrades| upgrades.get(id)) {
                Some(Tag::Int(level)) => (*level).max(0), _ => 0,
            };
            trait_level + upgrade_level
        };
        let level = modifier_level("tconstruct:vintage");
        if level > 0 {
            if !broken { values.push(("minecraft:movement_speed".to_owned(), format!("tconstruct:vintage_movement/{slot}"), -0.1 * f64::from(level), "add_multiplied_total")); }
            values.push(("minecraft:gravity".to_owned(), format!("tconstruct:vintage_gravity/{slot}"), 0.1 * f64::from(level), "add_multiplied_total"));
        }
        let heavy = modifier_level("tconstruct:heavy");
        if heavy > 0 {
            if !broken { values.push(("minecraft:movement_speed".to_owned(), format!("tconstruct:heavy_movement/{slot}"), -0.1 * f64::from(heavy), "add_multiplied_base")); }
            values.push(("minecraft:gravity".to_owned(), format!("tconstruct:heavy_gravity/{slot}"), 0.05 * f64::from(heavy), "add_multiplied_total"));
        }
        let featherweight = modifier_level("tconstruct:featherweight");
        let armor = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
            .and_then(|tags| tags.as_ref().ok())
            .and_then(|tags| tags.get("tconstruct:modifiable/armor"))
            .is_some_and(|items| items.contains(stack.item.as_ref()));
        if !broken && armor && featherweight > 0 {
            values.push(("minecraft:movement_speed".to_owned(), format!("tconstruct:featherweight_movement/{slot}"), 0.05 * f64::from(featherweight), "add_multiplied_base"));
        }
        let solid = modifier_level("tconstruct:solid");
        if solid > 0 {
            if !broken { values.push(("minecraft:movement_speed".to_owned(), format!("tconstruct:solid_movement/{slot}"), -0.1 * f64::from(solid), "add_multiplied_base")); }
            values.push(("minecraft:gravity".to_owned(), format!("tconstruct:solid_gravity/{slot}"), 0.05 * f64::from(solid), "add_multiplied_total"));
        }
        let entwined = modifier_level("tconstruct:entwined");
        if !broken && entwined > 0 {
            values.push(("minecraft:movement_speed".to_owned(), format!("tconstruct:entwined_movement/{slot}"), 0.1 * f64::from(entwined), "add_multiplied_total"));
        }
    }
    let Some(player) = game.player.as_mut() else { return; };
    for (attribute, id, amount, op) in values {
        if player.set_modifier(&attribute, &id, amount, op, true).is_ok() {
            game.applied_modifiers.push((attribute, id));
        }
    }
}
