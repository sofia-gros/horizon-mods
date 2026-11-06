// Adapted from official horizon-rust/horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0.
// Copyright (c) 2026 Ludovic Marie and the Horizon contributors. MIT.
// See HORIZON-DAMAGE-LICENSE.txt. Regenerate/review on every native body change.
// MIT License
//
// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
fn pluto_equipment_ordinary(
    game: &mut crate::game::Game<'_>,
    amount: f32,
    from: Option<[f64; 3]>,
    damage_type: &str,
    known: Option<Option<u32>>,
) -> std::ops::ControlFlow<bool> {
    if !horizon_client::health::pluto_protection::enabled()
        && !crate::game::pluto_block_feedback::enabled()
    {
        return std::ops::ControlFlow::Continue(());
    }
    fn original(
        game: &mut crate::game::Game<'_>,
        amount: f32,
        from: Option<[f64; 3]>,
        damage_type: &str,
        known: Option<Option<u32>>,
    ) -> bool {
        let post_attack_if_alive = std::mem::take(&mut game.combat.post_attack_if_alive);
        let rocket = std::mem::take(&mut game.combat.rocket_hit);
        // ServerPlayer.canHarmPlayer refused another player's projectile.
        if game.creative || game.combat.refuse_player_hit {
            game.combat.forget_noted(false);
            return false;
        }
        // A projectile entity dealt it: `#is_projectile`, or a firework
        // rocket (its `fireworks` type is not in the tag).
        let projectile = rocket
            || world_stations!(game)
                .enchantments
                .damage_tags
                .get("minecraft:is_projectile")
                .is_some_and(|t| t.contains(damage_type));
        let blocking = game.player_blocking(from, damage_type, false);
        let raised = blocking.is_some();
        // Melee: the attacker stands at `from` (another player's hit has no mob behind it).
        let attacker = match known {
            Some(known) => known,
            None => from
                .filter(|_| !projectile && damage_type != "minecraft:player_attack")
                .and_then(|from| {
                    game.mobs
                        .iter()
                        .filter(|m| m.alive())
                        .min_by(|a, b| {
                            let d = |m: &horizon_client::mobs::Mob| {
                                (m.position.x - from[0]).powi(2) + (m.position.z - from[2]).powi(2)
                            };
                            d(a).total_cmp(&d(b))
                        })
                        .map(|m| m.id)
                }),
        };
        // The causing entity for the death message.
        let owner = game.combat.projectile_owner.take();
        let _attacker = game.knockback_scope(
            attacker
                .or(owner)
                .map_or_else(horizon_client::health::knockback_attacker, |id| id as i32),
        );
        if let Some(a) = attacker.or(owner) {
            game.note_attacker(damage_type, a);
        }
        if let Some((id, name)) = game.combat.projectile_attacker.take() {
            game.note_named_attacker(damage_type, id, name);
        }
        // The projectile is the direct entity: named when it has no owner.
        if let Some(name) = game.combat.projectile_direct.take() {
            game.note_direct_entity(damage_type, name);
        }
        // DamageSource.scalesWithDifficulty: a mob (or its projectile's
        // owner) caused the hit, not a player.
        let living_cause = attacker.or(owner).is_some();
        // The native damage hook's causing entity: the one given, or the
        // owner of the projectile that hit (`projectile_owner` outlives a
        // projectile's other hits), not the guessed melee attacker.
        let owner = owner.filter(|_| projectile);
        let causing = known.flatten().or(owner).map_or(0, |id| id as i32);
        let mut outcome = horizon_client::health::BlockOutcome::default();
        // The hit's own push, held back until after the shield's (`hurtServer`
        // runs `applyItemBlocking` before `dealDefaultKnockback`).
        let mut default_push: Option<([f64; 2], HurtSource<'_>)> = None;
        let equipment = crate::game::pluto_damage_equipment::capture(game);
        if let Some(player) = game.player.as_mut()
            && let Some(id) = game.health_resources.damage.id(damage_type)
            && let Ok(o) = horizon_client::health::pluto_protection::hurt_with_optional_equipment(
                player,
                &game.health_resources,
                game.world.health_rules,
                id,
                amount,
                blocking,
                living_cause,
                causing,
                equipment.as_ref(),
            )
            && {
                outcome = o;
                o.hurt
            }
        {
            // LivingEntity.hurtServer's dealDefaultKnockback: not inside the
            // hurt cooldown (a stronger hit there only adds its difference).
            if o.knockback {
                // No source position: a random direction.
                let (xd, zd) = from.map_or((0.0, 0.0), |from| {
                    (from[0] - player.position.x, from[2] - player.position.z)
                });
                // dealDefaultKnockback's `indicateDamage`, unless blocked.
                if o.blocked <= 0.0 {
                    player.health.indicate_damage(xd, zd, player.yaw);
                }
                let source = HurtSource {
                    damage_type,
                    causing,
                    direct: 0,
                };
                default_push = Some(([xd, zd], source));
            }
            // The thorns hurt the entity that caused the hit, if there is one.
            let killed = game.player.as_ref().is_some_and(|p| {
                super::wither::killed(true, p.health.dead_or_dying(), p.health.totem_used)
            });
            if let Some(a) = attacker.or(owner)
                && !(post_attack_if_alive && killed)
            {
                game.thorns(a);
            }
        }
        // LivingEntity.hurtServer: ENTITY_HURT_PLAYER, unless it returned
        // early (invulnerable, or inside the hurt cooldown).
        // A hit a native mod canceled records no damage (hurtServer never ran).
        game.combat.forget_noted(outcome.canceled);
        if outcome.hurt || outcome.sound {
            let source = attacker
                .or(owner)
                .and_then(|id| game.mobs.iter().find(|m| m.id == id))
                .map(|m| game.mob_entity(m));
            game.trigger_entity_hurt_player(&horizon_client::advancements::DamageView {
                kind: damage_type.to_owned(),
                // Arrows and the like name no direct entity here; a
                // rocket's burst names the rocket.
                direct: if rocket {
                    Some(horizon_client::advancements::EntityView {
                        kind: "minecraft:firework_rocket".to_owned(),
                        position: from.unwrap_or_default(),
                        ..Default::default()
                    })
                } else {
                    source.clone().filter(|_| !projectile)
                },
                is_direct: !projectile,
                source,
                original: amount,
                actual: amount - outcome.blocked,
                blocked: outcome.blocked > 0.0,
            });
        }
        if raised {
            let source = HurtSource {
                damage_type,
                causing,
                direct: 0,
            };
            crate::game::pluto_block_feedback::dispatch(
                game,
                crate::game::pluto_block_feedback::Event {
                    outcome,
                    attacker,
                    projectile,
                    source,
                },
            );
            game.after_block(outcome, (attacker, projectile), source);
        }
        // `LivingEntity.dealDefaultKnockback`: after the item's block push, so
        // a half-blocked hit pushes the defender back the way vanilla does.
        if let Some(([xd, zd], source)) = default_push {
            game.knock_back_player(f64::from(0.4_f32), xd, zd, source);
            if let Some(p) = game.player.as_mut()
                && let Some(r) = p.hurt_log.last_mut()
            {
                r.knockback = Some([p.velocity.x, p.velocity.y, p.velocity.z]);
            }
        }
        outcome.hurt
    }
    std::ops::ControlFlow::Break(original(game, amount, from, damage_type, known))
}
