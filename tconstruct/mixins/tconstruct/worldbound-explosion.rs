// Adapted from the pinned native Game::explosion_hurt_entities.
// Keep the native entity walk, damage, callbacks and push order.
/*
MIT License

Copyright (c) 2026 Ludovic Marie and the Horizon contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

use super::*;

pub(super) fn try_hurt(
    game: &mut Game<'_>,
    center: [f64; 3],
    radius: f32,
    damage_type: &str,
    interaction: BlockInteraction,
    blocks: &[[i32; 3]],
) -> std::ops::ControlFlow<Vec<(u32, [f64; 3])>> {
    if game.remote.is_some() {
        return std::ops::ControlFlow::Continue(());
    }
    let damage = &game.motion.health.damage;
    let Some(id) = damage.id(damage_type) else {
        return std::ops::ControlFlow::Continue(());
    };
    // Source IndestructibleItemEntity still permits bypass-invulnerability
    // damage. Unknown types retain the original native behavior.
    if damage.in_tag(id, "minecraft:bypasses_invulnerability")
        || !game.dropped.iter().any(|item| {
            crate::game::tconstruct_worldbound_world::protects(game, &item.stack) == Some(true)
        })
    {
        return std::ops::ControlFlow::Continue(());
    }
    std::ops::ControlFlow::Break(hurt(game, center, radius, damage_type, interaction, blocks))
}

// The native body contains opaque entity-walk macros. A cancellable HEAD
// preserves it for ordinary blasts; this adapter changes only item destruction.
fn hurt(
    game: &mut Game<'_>,
    center: [f64; 3],
    radius: f32,
    damage_type: &str,
    interaction: BlockInteraction,
    blocks: &[[i32; 3]],
) -> Vec<(u32, [f64; 3])> {
    if radius < 1.0e-5 {
        return Vec::new();
    }
    game.explosion_hits_special(center, radius, blocks);
    let diameter = f64::from(radius * 2.0);
    let damage = |dist: f64, exposure: f32| {
        let pow = (1.0 - dist) * f64::from(exposure);
        ((pow * pow + pow) / 2.0 * 7.0 * diameter + 1.0) as f32
    };
    // `source.doTeamsAllowDamage(entity)`: the exploding mob spares its
    // team mates when its team has no friendly fire (they are still pushed).
    let exploding = game.combat.explosion_source;
    let teams_allow = |g: &Game<'_>, victim: Option<&horizon_client::scoreboard::Team>| {
        exploding
            .is_none_or(|id| horizon_client::scoreboard::teams_allow_damage(g.mob_team(id), victim))
    };
    // The player, then every other player in the level (LAN guests),
    // each swapped in as the player. In a guest's turn (their hit on an
    // end crystal) the player is that guest. The blast's source is the
    // same for all of them (`onExplosionHit(this.source)`).
    let attacker = game.blast_attacker();
    let blast_source = game.combat.explosion_cause.clone();
    let context = ExplosionHookContext { center, blocks };
    let own = game.explosion_hurt_player(
        &context,
        radius,
        damage_type,
        exploding,
        attacker.as_ref(),
        blast_source.as_ref(),
    );
    let mut pushed: Vec<_> = game.lan_acting.zip(own).into_iter().collect();
    if let Some(mut host) = game.lan.take() {
        let guests: Vec<u32> = host
            .hub
            .guests
            .iter()
            .filter(|g| host.hub.here(g) && g.loaded && !g.dead)
            .map(|g| g.conn)
            .collect();
        for conn in guests {
            let push = game.as_guest(&mut host, conn, |g| {
                g.explosion_hurt_player(
                    &context,
                    radius,
                    damage_type,
                    exploding,
                    attacker.as_ref(),
                    blast_source.as_ref(),
                )
            });
            if let Some(Some(push)) = push {
                pushed.push((conn, push));
            }
        }
        game.lan = Some(host);
    }
    // Mobs. `WitherBoss.hurtServer`: a wither shrugs off an undead cause.
    let cause = game
        .combat
        .explosion_owner
        .or(exploding)
        .and_then(|id| game.mobs.iter().find(|m| m.id == id))
        .map(|m| m.species());
    // `ServerExplosion.hurtEntities`: every entity in reach, on the walk. Each
    // answers `ignoreExplosion` itself (`Entity::ignores_explosion`); the rest
    // is the same for all, and only what a blast does to a kind differs. The
    // walk covers the entities there are now: what the blast drops or spawns
    // is not hit by it. Destroyed items and orbs are squeezed out afterwards
    // (as `retain` does), and a destroyed boat or cart shifts its list, so
    // `gone` counts how far.
    let blast = Blast {
        vehicles_ignore: game.combat.blast_owner.is_none()
            && game.blast_attacker().is_some()
            && !game.ui.rules.bool("mob_griefing"),
        items_ignore: items_ignore_explosion(game.ui.rules.bool("mob_griefing"), interaction),
        spared: exploding.filter(|_| game.combat.explosion_source_named),
    };
    let (items_len, orbs_len) = (game.dropped.len(), game.orbs.len());
    let (mut items_kept, mut orbs_kept) = (0, 0);
    let (mut boats_gone, mut carts_gone) = (0, 0);
    for handle in game.entity_handles() {
        let handle = match handle {
            EntityHandle::Boat(i) => EntityHandle::Boat(i - boats_gone),
            EntityHandle::Cart(i) => EntityHandle::Cart(i - carts_gone),
            other => other,
        };
        let hit = with_entity!(game, handle, e => game.blast_hit(e, center, diameter, &blast));
        let mut destroyed = false;
        if let Some(Hit {
            dist,
            exposure,
            push,
        }) = hit
        {
            let amount = damage(dist, exposure);
            let entity = with_entity!(game, handle, e => e.id());
            match handle {
                EntityHandle::Mob(i) => {
                    let hurtable = teams_allow(game, game.mob_team(game.mobs[i].id));
                    let m = &mut game.mobs[i];
                    // The exploding mob is the damage source's entity, so
                    // victims it kills drop its charged-creeper loot
                    // (`Creeper.killedEntity`).
                    let source = horizon_client::mobs::Damage {
                        kind: horizon_client::mobs::DamageKind::Explosion,
                        // A projectile's owner (`getIndirectSourceEntity`),
                        // else the exploding mob itself.
                        attacker: attacker
                            .as_ref()
                            .map(|&(id, _)| id)
                            .filter(|&o| o != m.id)
                            .map(horizon_client::mobs::Who::Mob),
                        from: Some(center),
                    };
                    // `EXPLOSION_KNOCKBACK_RESISTANCE`, not `KNOCKBACK_RESISTANCE`:
                    // its base is 0 for every mob type, so iron golems and
                    // ravagers are pushed.
                    hurt_mob(m, hurtable, cause, (amount, source));
                }
                // 5 health, destroyed by the explosion damage (nether stars resist).
                EntityHandle::Item(i) => {
                    destroyed =
                        &*game.dropped[i].stack.item != "minecraft:nether_star" && amount >= 5.0;
                    if crate::game::tconstruct_worldbound_world::protects(
                        game,
                        &game.dropped[i].stack,
                    ) == Some(true)
                    {
                        destroyed = false;
                    }
                }
                // `ExperienceOrb.hurtServer` takes the damage off their 5 health.
                EntityHandle::Orb(i) => {
                    destroyed = game.orbs[i].hurt(amount);
                }
                // `VehicleEntity.hurtServer` (explosions ignite TNT carts);
                // a destroyed one is gone before the push.
                EntityHandle::Cart(i) => {
                    let len = game.carts.len();
                    game.hurt_cart(i, amount, false, true);
                    carts_gone += len - game.carts.len();
                }
                // `VehicleEntity.hurtServer`: damage * 10, broken past 40.
                EntityHandle::Boat(i) => {
                    let len = game.boats.len();
                    game.hurt_boat(i, amount, false);
                    boats_gone += len - game.boats.len();
                }
                // `hurtServer` does nothing to the rest.
                _ => {}
            }
            let mut velocity = [push.x, push.y, push.z];
            game.native_explosion_knockback(
                game.dim.id(),
                center,
                blocks,
                Some(entity),
                &mut velocity,
            );
            let push = Vec3::new(velocity[0], velocity[1], velocity[2]);
            match handle {
                EntityHandle::Item(_) | EntityHandle::Orb(_) if destroyed => {}
                EntityHandle::Cart(_) => {
                    if let Some(c) = game.carts.iter_mut().find(|c| c.id == entity) {
                        c.push_from_explosion(push);
                    }
                }
                EntityHandle::Boat(_) => {
                    if let Some(b) = game.boats.iter_mut().find(|b| b.id == entity) {
                        b.push_from_explosion(push);
                    }
                }
                other => with_entity_mut!(game, other, e => e.push_from_explosion(push)),
            }
        }
        match handle {
            EntityHandle::Item(i) if !destroyed => {
                game.dropped.swap(items_kept, i);
                items_kept += 1;
            }
            EntityHandle::Orb(i) if !destroyed => {
                game.orbs.swap(orbs_kept, i);
                orbs_kept += 1;
            }
            _ => {}
        }
    }
    game.dropped.drain(items_kept..items_len);
    game.orbs.drain(orbs_kept..orbs_len);
    game.native_leaves_flush();
    // Frames, paintings, cushions and armour stands last: vanilla lists
    // the entities in reach before hurting any, so the items they drop
    // here are neither destroyed nor pushed by the same blast.
    game.explosion_breaks_frames(center, diameter, interaction);
    game.explosion_breaks_decorations(center, diameter, interaction);
    pushed
}
