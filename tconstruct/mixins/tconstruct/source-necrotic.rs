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
    let Some(amount) = heal_amount(level, dealt, scale, crit, landed) else { return; };
    heal_player(game, amount);
}

fn heal_player(game: &mut super::Game<'_>, mut amount: f32) {
    let Some(player) = game.player.as_mut() else { return; };
    if player.health.dead_or_dying() { return; }
    let current = player.health.health();
    if !current.is_finite() || !amount.is_finite() { return; }
    let mut cancel = false;
    horizon_client::hunger::native_living_heal(player.entity_id as u32, &mut amount, &mut cancel);
    if !cancel && amount.is_finite() && amount > 0.0 { let _ = player.set_health(current + amount, false); }
    let at = [player.position.x, player.position.y, player.position.z];
    game.audio.play("tconstruct:necrotic_heal", Some(at), 1.0, 1.0);
}

// A bounded handoff connects the pre-hit health with the outcome for the same
// tracked arrow and acting player. A subsequent hit invalidates stale entries.
pub(super) fn projectile_begin(
    game: &mut super::Game<'_>,
    p: &horizon_client::projectiles::Projectile,
    hit: horizon_client::projectiles::Hit,
) {
    use horizon_client::{
        mobs::Who,
        projectiles::{Hit, ProjectileKind},
    };
    pvp_capture(game, p, hit);
    let actor = game.lan_acting;
    game.world
        .chunks
        .__horizon_mixins_tconstruct_necrotic_projectile_mut()
        .remove(&actor);
    if game.remote.is_some() || p.id == 0 || (p.guest.is_some() && p.guest != actor) {
        return;
    }
    // Loaded arrows retain the owner's UUID even when claim_owner cannot
    // reconstruct a host-player Who. Match the current actor exactly.
    let owned = p.owner == Some(Who::Player)
        || (p.owner.is_none() && p.owner_uuid.is_some_and(|id| id == game.acting_player_uuid()));
    if !owned { return; }
    let Hit::Entity {
        who: Who::Mob(id), ..
    } = hit
    else {
        return;
    };
    let ProjectileKind::Arrow { base_damage, .. } = &p.kind else {
        return;
    };
    let Some(mob) = game.mobs.iter().find(|m| m.id == id) else {
        return;
    };
    let kind = if mob.kind.id.contains(':') {
        mob.kind.id.to_string()
    } else {
        format!("minecraft:{}", mob.kind.id)
    };
    let data = game.res.mob_data();
    if !data
        .entity_tags
        .contains_key("tconstruct:necrotic_blacklist")
        || data.entity_has("tconstruct:necrotic_blacklist", &kind)
    {
        return;
    }
    let Some(weapon) = p.arrow.as_ref().and_then(|a| a.fired_from_weapon.as_ref()) else {
        return;
    };
    let Some(traits) = super::tconstruct_source_material_traits::levels(&game.world, weapon) else {
        return;
    };
    let level = traits
        .levels
        .get("tconstruct:necrotic")
        .copied()
        .unwrap_or(0);
    let speed =
        (p.velocity.x * p.velocity.x + p.velocity.y * p.velocity.y + p.velocity.z * p.velocity.z)
            .sqrt();
    let Some(amount) = projectile_amount(level, mob.health, *base_damage, speed) else {
        return;
    };
    game.world
        .chunks
        .__horizon_mixins_tconstruct_necrotic_projectile_mut()
        .insert(actor, (p.id, amount));
}

fn projectile_amount(level: i32, health: f32, base: f64, speed: f64) -> Option<f32> {
    if level <= 0 || !health.is_finite() || health <= 0.0 || !base.is_finite() || !speed.is_finite()
    {
        return None;
    }
    let power = (base * speed).clamp(0.0, i32::MAX as f64).ceil() as f32;
    if power <= 0.0 {
        return None;
    }
    Some(0.05 * level as f32 * health.min(power))
}

pub(super) fn projectile_finish(
    game: &mut super::Game<'_>,
    p: &horizon_client::projectiles::Projectile,
    outcome: horizon_client::arrows::EntityHit,
) {
    pvp_outcome(game, p, outcome);
    let actor = game.lan_acting;
    let pending = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_necrotic_projectile_mut()
        .remove(&actor);
    if outcome != horizon_client::arrows::EntityHit::Hurt
        || p.guest.is_some_and(|guest| Some(guest) != actor)
    {
        return;
    }
    if let Some((_, amount)) = pending.filter(|(id, _)| *id == p.id) {
        heal_player(game, amount);
    }
}

// A scope per host PvP call; nested calls invalidate their parents rather
// than attributing an inner victim or outcome to an outer projectile.
pub(super) fn pvp_begin(game: &mut super::Game<'_>, p: &horizon_client::projectiles::Projectile) {
    use horizon_client::mobs::Who;
    let owner = p.owner_uuid;
    let valid = if game.remote.is_some() || p.id == 0 || p.owner != Some(Who::Player) || owner.is_none() {
        false
    } else if let Some(conn) = p.guest {
        game.lan.as_ref().and_then(|host| host.hub.guest(conn))
            .is_some_and(|guest| Some(horizon_client::lan::playerdata::uuid_ints(guest.uuid)) == owner)
    } else {
        owner == Some(game.acting_player_uuid())
    };
    let contexts = game.world.chunks.__horizon_mixins_tconstruct_necrotic_pvp_mut();
    for parent in contexts.iter_mut() { *parent = None; }
    contexts.push(if valid { owner.map(|uuid| (p.id, p.guest, uuid, None, false)) } else { None });
}

fn pvp_capture(game: &mut super::Game<'_>, p: &horizon_client::projectiles::Projectile, hit: horizon_client::projectiles::Hit) {
    use horizon_client::{mobs::Who, projectiles::{Hit, ProjectileKind}};
    if !matches!(hit, Hit::Entity { who: Who::Player, .. }) { return; }
    let pending = game.world.chunks.__horizon_mixins_tconstruct_necrotic_pvp().last().and_then(Option::as_ref).copied();
    let Some((id, shooter, uuid, _, _)) = pending else { return; };
    if id != p.id || shooter != p.guest || Some(uuid) != p.owner_uuid || shooter == game.lan_acting { return; }
    let data = game.res.mob_data();
    if !data.entity_tags.contains_key("tconstruct:necrotic_blacklist") || data.entity_has("tconstruct:necrotic_blacklist", "minecraft:player") { return; }
    let ProjectileKind::Arrow { base_damage, .. } = &p.kind else { return; };
    let Some(health) = game.player.as_ref().map(|player| player.health.health()) else { return; };
    let Some(weapon) = p.arrow.as_ref().and_then(|a| a.fired_from_weapon.as_ref()) else { return; };
    let Some(traits) = super::tconstruct_source_material_traits::levels(&game.world, weapon) else { return; };
    let level = traits.levels.get("tconstruct:necrotic").copied().unwrap_or(0);
    let speed = (p.velocity.x*p.velocity.x + p.velocity.y*p.velocity.y + p.velocity.z*p.velocity.z).sqrt();
    let Some(amount) = projectile_amount(level, health, *base_damage, speed) else { return; };
    if let Some(Some(entry)) = game.world.chunks.__horizon_mixins_tconstruct_necrotic_pvp_mut().last_mut() {
        if entry.0 == id && entry.1 == shooter && entry.2 == uuid { entry.3 = Some(amount); }
    }
}

fn pvp_outcome(game: &mut super::Game<'_>, p: &horizon_client::projectiles::Projectile, outcome: horizon_client::arrows::EntityHit) {
    if let Some(Some(entry)) = game.world.chunks.__horizon_mixins_tconstruct_necrotic_pvp_mut().last_mut() {
        if entry.0 == p.id && entry.1 == p.guest && Some(entry.2) == p.owner_uuid && entry.3.is_some() {
            entry.4 = outcome == horizon_client::arrows::EntityHit::Hurt;
        }
    }
}

pub(super) fn pvp_end(game: &mut super::Game<'_>, p: &horizon_client::projectiles::Projectile, succeeded: bool) {
    let pending = game.world.chunks.__horizon_mixins_tconstruct_necrotic_pvp_mut().pop().flatten();
    let Some((id, shooter, uuid, Some(amount), true)) = pending else { return; };
    if !succeeded || id != p.id || shooter != p.guest || Some(uuid) != p.owner_uuid { return; }
    if let Some(conn) = shooter {
        let Some(mut host) = game.lan.take() else { return; };
        let matches = host.hub.guest(conn).is_some_and(|g| horizon_client::lan::playerdata::uuid_ints(g.uuid) == uuid);
        if matches { let _ = game.as_guest(&mut host, conn, |actor| heal_player(actor, amount)); }
        game.lan = Some(host);
    } else if game.acting_player_uuid() == uuid {
        heal_player(game, amount);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projectile_uses_pre_hit_health_and_ceiled_arrow_power() {
        assert_eq!(projectile_amount(2, 20.0, 2.0, 1.1), Some(0.3));
        assert_eq!(projectile_amount(1, 2.0, 10.0, 3.0), Some(0.1));
        assert_eq!(projectile_amount(1, 2.0, 0.0, 3.0), None);
        assert_eq!(projectile_amount(0, 2.0, 1.0, 3.0), None);
    }
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
