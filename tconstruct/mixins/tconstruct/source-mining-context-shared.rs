// Apply source conditional break-speed modules after efficiency and potions.
pub(super) fn apply(speed: f32, game: &crate::game::Game<'_>, facts: &&horizon_items::blocks::StateFacts, held: &Option<&horizon_client::items::ItemStack>, position: Option<(i32, i32)>) -> f32 {
    let armor_multiplier = game.armor[1].as_ref().and_then(|stack| {
        let custom = stack.data()?.other("minecraft:custom_data")?;
        // A client uses only the matching actor-specific authoritative frame.
        // A server uses the acting player's swapped equipment and live world.
        let traits = if game.remote.is_some() {
            crate::game::tconstruct_source_haste_wire::matching_level(
                game.world.chunks.__horizon_mixins_tconstruct_haste_wire_client().as_ref(), Some(stack)).unwrap_or(0)
        } else {
            (|| {
                let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
                let harvest = tags.get("tconstruct:modifiable/harvest")?;
                let raw = game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()?.as_ref().ok()?;
                let source = crate::game::tconstruct_modifier_snapshot::highest_definition(raw, "tconstruct:tinkering/modifiers/haste");
                if !crate::game::tconstruct_source_haste_source::source_haste_trait_allowed(Some(harvest.contains(&*stack.item)), source) { return None; }
                Some(crate::game::tconstruct_source_material_traits::levels(&game.world, stack)?.levels.get("tconstruct:haste").copied().unwrap_or(0))
            })().unwrap_or(0)
        };
        Some(crate::game::tconstruct_source_mining_rules::haste_armor_multiplier_with_trait(custom, stack.damage, traits))
    }).unwrap_or(1.0);
    let Some(stack) = held.filter(|stack| stack.item.starts_with("tconstruct:")) else { return speed * armor_multiplier; };
    let Some(data) = stack.data() else { return speed * armor_multiplier; };
    let Some(custom) = data.other("minecraft:custom_data") else { return speed * armor_multiplier; };
    let effective = match data.other("minecraft:tool") {
        Some(tool) => match crate::game::tconstruct_source_mining_rules::evaluated_speed(tool, &facts.name, |tag, block| game.items().block_in_tag(tag, block)) {
            Some(value) => value.is_some(),
            None => return speed * armor_multiplier,
        },
        None => false,
    };
    let broad_tag = if game.remote.is_some() {
        game.world.chunks.__horizon_mixins_tconstruct_broad_wire_client().as_ref()
    } else {
        game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
            .and_then(|result| result.as_ref().ok())
            .and_then(|tags| tags.get("tconstruct:modifiable/broad"))
    };
    let broad = crate::game::tconstruct_source_mining_rules::broad_or_unknown(broad_tag, &stack.item);
    // Source BreakSpeedContext supplies potion and environmental factors to
    // flat bonuses. Host base speed may use different fatigue/water rules.
    let water = game.player.as_ref().map(horizon_client::pluto_fluid_facts::player);
    let in_water = water.is_some_and(|facts| facts.eye_in_water);
    let aqua_affinity = game.armor[0].as_ref().is_some_and(|stack| stack.enchantment_level("minecraft:aqua_affinity") > 0);
    let on_ground = game.player.as_ref().map_or(true, |player| player.ground.on_ground);
    let effects = &game.potions.effects;
    let multiplier = crate::game::tconstruct_source_mining_rules::environment_multiplier(
        effects.amplifier("haste"), effects.amplifier("conduit_power"),
        effects.amplifier("mining_fatigue"), in_water, aqua_affinity, on_ground);
    // Mantle's eyes_in_water predicate uses the actor's last eye-water sample,
    // which can differ from its current eye-in-fluid state.
    let hydraulic_water = water.and_then(|facts|
        facts.previous_eye_in_water.then_some(aqua_affinity));
    let speed = crate::game::tconstruct_source_mining_rules::conditional_with_water(custom, stack.damage, speed, multiplier, position, effective, !facts.requires_tool, broad, Some(facts.explosion_resistance), hydraulic_water).unwrap_or(speed);
    crate::game::tconstruct_source_mining_rules::finish_mining_speed(speed * crate::game::tconstruct_source_mining_rules::airborne_multiplier(custom, stack.damage, on_ground), armor_multiplier)
}
