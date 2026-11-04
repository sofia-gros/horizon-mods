fn tconstruct_material_armor(game: &mut crate::game::Game<'_>) {
    use horizon_world::nbt::Tag;
    let mut values = Vec::new();
    // LevelingValue computes the default flat zero plus coefficient * effective
    // level in float; AttributeModule widens that result for the attribute API.
    let leveled_amount = |coefficient: f32, level: i32| f64::from(0.0f32 + coefficient * level as f32);
    for (stack, slot) in game.armor.iter().zip(["head", "chest", "legs", "feet"]) {
        let Some(stack) = stack.as_ref().filter(|stack| stack.item.starts_with("tconstruct:")) else { continue; };
        let broken = crate::game::tconstruct_source_ranged_wear::is_broken(stack);
        let Some(data) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { continue; };
        // Pinned ModifiableArmorItem grants base stats only in its class slot.
        // These stock class slots cannot be reassigned by mutable item tags or
        // equippable components. External item classes remain unsupported here.
        let armor_slot = match stack.item.as_ref() {
            "tconstruct:travelers_helmet" | "tconstruct:plate_helmet" | "tconstruct:slime_helmet" => Some("head"),
            "tconstruct:travelers_chestplate" | "tconstruct:plate_chestplate" | "tconstruct:slimy_chestplate" | "tconstruct:slime_wings" => Some("chest"),
            "tconstruct:travelers_leggings" | "tconstruct:plate_leggings" | "tconstruct:slime_leggings" => Some("legs"),
            "tconstruct:travelers_boots" | "tconstruct:plate_boots" | "tconstruct:slime_boots" => Some("feet"),
            _ => None,
        };
        let attributes = !broken && armor_slot == Some(slot);
        if attributes {
            for (key, attribute) in [("tic_armor", "minecraft:armor"), ("tic_armor_toughness", "minecraft:armor_toughness"), ("tic_knockback_resistance", "minecraft:knockback_resistance")] {
                if let Some(Tag::Float(bits)) = data.get(key) {
                    let amount = f32::from_bits(*bits);
                    // Pinned base armor stats contribute only when positive.
                    // This also excludes NaN, without changing +infinity policy.
                    if amount > 0.0 {
                        values.push((attribute.to_owned(), format!("tconstruct:{key}/{slot}"), f64::from(amount), "add_value"));
                    }
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
            if attributes { values.push(("minecraft:movement_speed".to_owned(), format!("tconstruct:vintage_movement/{slot}"), leveled_amount(-0.1, level), "add_multiplied_total")); }
            values.push(("minecraft:gravity".to_owned(), format!("tconstruct:vintage_gravity/{slot}"), leveled_amount(0.1, level), "add_multiplied_total"));
        }
        let heavy = modifier_level("tconstruct:heavy");
        if heavy > 0 {
            if attributes { values.push(("minecraft:movement_speed".to_owned(), format!("tconstruct:heavy_movement/{slot}"), leveled_amount(-0.1, heavy), "add_multiplied_base")); }
            values.push(("minecraft:gravity".to_owned(), format!("tconstruct:heavy_gravity/{slot}"), leveled_amount(0.05, heavy), "add_multiplied_total"));
        }
        let featherweight = modifier_level("tconstruct:featherweight");
        let armor = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
            .and_then(|tags| tags.as_ref().ok())
            .and_then(|tags| tags.get("tconstruct:modifiable/armor"))
            .is_some_and(|items| items.contains(stack.item.as_ref()));
        if attributes && armor && featherweight > 0 {
            values.push(("minecraft:movement_speed".to_owned(), format!("tconstruct:featherweight_movement/{slot}"), leveled_amount(0.05, featherweight), "add_multiplied_base"));
        }
        let solid = modifier_level("tconstruct:solid");
        if solid > 0 {
            if attributes { values.push(("minecraft:movement_speed".to_owned(), format!("tconstruct:solid_movement/{slot}"), leveled_amount(-0.1, solid), "add_multiplied_base")); }
            values.push(("minecraft:gravity".to_owned(), format!("tconstruct:solid_gravity/{slot}"), leveled_amount(0.05, solid), "add_multiplied_total"));
        }
        let entwined = modifier_level("tconstruct:entwined");
        if attributes && entwined > 0 {
            values.push(("minecraft:movement_speed".to_owned(), format!("tconstruct:entwined_movement/{slot}"), leveled_amount(0.1, entwined), "add_multiplied_total"));
        }
        let speedy_trait = traits.as_ref().and_then(|traits| traits.levels.get("tconstruct:speedy").copied()).unwrap_or(0).max(0);
        let speedy_progress = match data.get("tic_upgrades").and_then(|upgrades| upgrades.get("tconstruct:speedy")) {
            Some(Tag::Int(progress)) => (*progress).max(0), _ => 0,
        };
        // Source merges integer logical levels before constructing the partial
        // level. scaleLevel adds a negative fraction to the rounded logical
        // level, rather than dividing total progress directly.
        let remainder = speedy_progress % 45;
        let speedy = speedy_trait.checked_add(speedy_progress / 45).and_then(|complete| {
            if remainder == 0 { Some(complete as f32) }
            else { complete.checked_add(1).map(|logical| logical as f32 + (remainder - 45) as f32 / 45.0f32) }
        });
        // Unrepresentable merged logical levels fail closed for Speedy only.
        if let Some(speedy) = speedy.filter(|level| attributes && *level > 0.0) {
            values.push(("minecraft:movement_speed".to_owned(), format!("tconstruct:speedy_movement/{slot}"), f64::from(0.0f32 + 0.1f32 * speedy), "add_multiplied_total"));
        }
        let step_up = modifier_level("tconstruct:step_up");
        if attributes && step_up > 0 {
            values.push(("minecraft:step_height".to_owned(), format!("tconstruct:step_up/{slot}"), leveled_amount(0.5, step_up), "add_value"));
        }
    }
    let Some(player) = game.player.as_mut() else { return; };
    for (attribute, id, amount, op) in values {
        if player.set_modifier(&attribute, &id, amount, op, true).is_ok() {
            game.applied_modifiers.push((attribute, id));
        }
    }
}
