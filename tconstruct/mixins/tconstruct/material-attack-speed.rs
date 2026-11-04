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
    // Broken source tools supply no held attribute modifier. Do not fall
    // through to the host's static registry item modifier for this stack.
    let broken = crate::game::tconstruct_source_ranged_wear::is_broken(stack);
    let item_modifier = if broken {
        0.0
    } else {
        f64::from(f32::from_bits(*bits)) - horizon_client::attack_strength::BASE_ATTACK_SPEED
    };
    // Source Overwield is a live mainhand multiply_total attribute, not a
    // permanent tool-stat boost. Re-read overslime and merged levels each call.
    let overwield = if broken || game.hand != horizon_client::first_person::Hand::Main { 0.0 } else {
        stack.data().and_then(|data| data.other("minecraft:custom_data"))
            .and_then(|custom| match custom.get("tic_overslime") { Some(Tag::Int(n)) if *n > 0 => Some(*n), _ => None })
            .and_then(|amount| {
                let raw = game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()?.as_ref().ok()?;
                let source = crate::game::tconstruct_modifier_snapshot::highest_definition(raw, "tconstruct:tinkering/modifiers/overwield");
                if !crate::game::tconstruct_modifier_snapshot::known_overwield_attribute(source) { return None; }
                let traits = crate::game::tconstruct_source_material_traits::levels(&game.world, stack)?;
                let level = crate::game::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, "tconstruct:overwield")?;
                Some(f64::from(amount.min(level.max(0)) as f32 * 0.15f32))
            }).unwrap_or(0.0)
    };
    let value = base + item_modifier;
    let speed = game.potions.effects.modifiers().into_iter()
        .filter(|modifier| modifier.0 == "minecraft:attack_speed")
        .fold(value, |speed, modifier| speed * (1.0 + modifier.2));
    let speed = (speed * (1.0 + overwield)).clamp(0.0, 1024.0);
    Break(speed)
}
