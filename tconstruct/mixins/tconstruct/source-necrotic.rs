// Source Necrotic melee lifesteal; each actor owns a stack of attack scopes.

pub(super) fn heal_amount(level: i32, dealt: f32, scale: f32, crit: bool, landed: bool) -> Option<f32> {
    if level <= 0 || !landed || !crit || !scale.is_finite() || scale <= 0.9 || !dealt.is_finite() || dealt <= 0.0 { return None; }
    Some(0.05 * level as f32 * dealt)
}

pub(super) fn begin(game: &mut super::Game<'_>, index: usize) {
    if game.remote.is_some() { return; }
    let actor = game.lan_acting;
    let entry = game.mobs.get(index).zip(game.held()).and_then(|(mob, stack)| {
        if !stack.item.starts_with("tconstruct:") || super::tconstruct_source_melee_shape::intact(stack).is_none() { return None; }
        Some((mob.id, stack.clone()))
    });
    let contexts = game.world.chunks.__horizon_mixins_tconstruct_necrotic_context_mut();
    let pending = contexts.entry(actor).or_default();
    for parent in pending.iter_mut() { *parent = None; }
    pending.push(entry);
}

pub(super) fn end(game: &mut super::Game<'_>) {
    if game.remote.is_some() { return; }
    let actor = game.lan_acting;
    let contexts = game.world.chunks.__horizon_mixins_tconstruct_necrotic_context_mut();
    let Some(entries) = contexts.get_mut(&actor) else { return; };
    entries.pop();
    if entries.is_empty() { contexts.remove(&actor); }
}

pub(super) fn hit(game: &mut super::Game<'_>, index: usize) {
    if game.remote.is_some() { return; }
    let actor = game.lan_acting;
    let contexts = game.world.chunks.__horizon_mixins_tconstruct_necrotic_context_mut();
    let Some((target, mut before)) = contexts.get_mut(&actor).and_then(|entries| entries.last_mut()).and_then(Option::take) else { return; };
    let Some(mob) = game.mobs.get(index).filter(|mob| mob.id == target) else { return; };
    let kind = if mob.kind.id.contains(':') { mob.kind.id.to_string() } else { format!("minecraft:{}", mob.kind.id) };
    let mob_data = game.res.mob_data();
    if !mob_data.entity_tags.contains_key("tconstruct:necrotic_blacklist") || mob_data.entity_has("tconstruct:necrotic_blacklist", &kind) { return; }
    let Some(record) = game.player.as_ref().and_then(|player| player.attack_log.last()) else { return; };
    if record.id != target { return; }
    let (dealt, scale, crit, landed) = (record.dealt, record.scale, record.crit, record.landed);
    let Some(stack) = game.held().cloned() else { return; };
    before.damage = stack.damage;
    if before.count != stack.count || stack.count != 1 || !before.same_item_same_components(&stack) || stack.wire_patch().is_some() || super::tconstruct_source_melee_shape::intact(&stack).is_none() { return; }
    let Some(traits) = super::tconstruct_source_material_traits::levels(&game.world, &stack) else { return; };
    let level = traits.levels.get("tconstruct:necrotic").copied().unwrap_or(0);
    let Some(mut amount) = heal_amount(level, dealt, scale, crit, landed) else { return; };
    let Some(player) = game.player.as_mut() else { return; };
    if player.health.dead_or_dying() { return; }
    let current = player.health.health();
    if !current.is_finite() || !amount.is_finite() { return; }
    let mut cancel = false;
    horizon_client::hunger::native_living_heal(player.entity_id as u32, &mut amount, &mut cancel);
    if !cancel && amount.is_finite() && amount > 0.0 { let _ = player.set_health(current + amount, false); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn melee_heal_requires_landed_charged_critical_and_scales_damage() {
        assert_eq!(heal_amount(2, 10.0, 1.0, true, true), Some(1.0));
        assert_eq!(heal_amount(1, 4.0, 1.0, true, true), Some(0.2));
        assert_eq!(heal_amount(1, 4.0, 0.9, true, true), None);
        assert_eq!(heal_amount(1, 4.0, 1.0, false, true), None);
        assert_eq!(heal_amount(1, 4.0, 1.0, true, false), None);
        assert_eq!(heal_amount(1, 0.0, 1.0, true, true), None);
    }
}
