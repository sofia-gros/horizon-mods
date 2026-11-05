fn tconstruct_material_melee_bonus(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<f32> {
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    if let Some(damage) = crate::game::tconstruct_offhand_attack_world::damage(game) { return Break(damage); }
    // Attack attributes belong to the selected mainhand, not the acting hand.
    let Some(stack) = game.inventory.get(game.selected).and_then(Option::as_ref)
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
    // Source broken tools provide no held attack-damage modifier. Keep the
    // projection so the host's static item damage is removed rather than
    // falling through and reintroducing it. Percent wear traits also stop.
    let broken = crate::game::tconstruct_source_ranged_wear::is_broken(stack);
    // Only the base held stat requires MELEE_WEAPON. Modifier effects below
    // remain independent; retain the clone projection to remove native damage.
    let melee_weapon = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
        .and_then(|tags| tags.as_ref().ok())
        .and_then(|tags| tags.get("tconstruct:modifiable/melee/weapon"))
        .is_some_and(|items| items.contains(&*stack.item));
    let damage = if broken || !melee_weapon { 0.0 } else { f32::from_bits(*bits) };
    // Conditional melee wear modifiers require MELEE, independently of the
    // narrower MELEE_WEAPON requirement for the base attribute above.
    let melee = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
        .and_then(|tags| tags.as_ref().ok())
        .and_then(|tags| tags.get("tconstruct:modifiable/melee"))
        .is_some_and(|items| items.contains(&*stack.item));
    let trait_bonus = (!broken && melee).then(|| crate::game::tconstruct_source_material_traits::levels(&game.world, stack)).flatten()
        .map(|traits| {
            let raw = game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref().and_then(|raw| raw.as_ref().ok());
            let wear_level = |id| {
                if crate::game::tconstruct_modifier_snapshot::known_melee_wear(raw, id) {
                    traits.levels.get(id).copied().unwrap_or(0)
                } else { 0 }
            };
            let jagged = wear_level("tconstruct:jagged");
            let stonebound = wear_level("tconstruct:stonebound");
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
