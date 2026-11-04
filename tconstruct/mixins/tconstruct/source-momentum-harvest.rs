// Called after successful non-creative removal and before the tool takes wear.
pub(super) fn apply(game: &mut crate::game::Game<'_>, block: &str, hardness: f32) {
    if game.remote.is_some() { return; }
    let actor = game.lan_acting;
    if game.world.chunks.__horizon_mixins_tconstruct_area_active().contains(&actor) {
        if !game.world.chunks.__horizon_mixins_tconstruct_momentum_area_seen_mut().insert(actor) { return; }
    }
    let Some(stack) = game.held() else { return; };
    if crate::game::tconstruct_source_ranged_wear::is_broken(stack) { return; }
    let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()) else { return; };
    if !tags.get("tconstruct:modifiable/harvest").is_some_and(|tag| tag.contains(stack.item.as_ref())) { return; }
    let Some(data) = stack.data() else { return; };
    let Some(custom) = data.other("minecraft:custom_data") else { return; };
    if crate::game::tconstruct_source_momentum_rules::trait_level(custom) == 0 { return; }
    let Some(tool) = data.other("minecraft:tool") else { return; };
    let Some(Some(speed)) = crate::game::tconstruct_source_mining_rules::evaluated_speed(tool, block, |tag, id| game.items().block_in_tag(tag, id)) else { return; };
    let Some((captured, requires_tool)) = game.world.chunks.__horizon_mixins_tconstruct_momentum_break_context().get(&actor) else { return; };
    if captured != block || (*requires_tool && !game.res.items().correct_for_drops(Some(stack), block)) { return; }
    let duration = crate::game::tconstruct_source_momentum_rules::harvest_duration(hardness, speed);
    let Some(def) = horizon_client::brewing::effect("tconstruct:momentum_harvest") else { return; };
    let amplifier = game.potions.effects.amplifier(def.id).map_or(0, |n| n.saturating_add(1).min(9));
    let mut effect = horizon_client::effects::EffectInstance::new(def.id, duration, amplifier);
    effect.visible = false; // Hide particles; the host protocol still sets the HUD-icon flag.
    game.add_player_effect(effect);
}

// Called only for a real primary ranged projectile after critical resolution.
pub(super) fn add_ranged(game: &mut crate::game::Game<'_>) {
    if game.remote.is_some() { return; }
    let Some(def) = horizon_client::brewing::effect("tconstruct:momentum_ranged") else { return; };
    let amplifier = game.potions.effects.amplifier(def.id).map_or(0, |n| n.saturating_add(1).min(9));
    let mut effect = horizon_client::effects::EffectInstance::new(def.id, 200, amplifier);
    effect.visible = false;
    game.add_player_effect(effect);
}
