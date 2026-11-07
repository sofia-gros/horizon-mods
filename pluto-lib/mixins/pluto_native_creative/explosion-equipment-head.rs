// Adapted from horizon-rust/horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0,
// crates/horizon-client/src/game/explosion.rs::Game::explosion_hurt_player.
// Only receiver renaming, equipment capture, low-level damage dispatch and
// final block feedback differ in `original`. Source/complete-plan guards are
// required before applying this lowest-priority cancellable HEAD adapter.
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
fn pluto_explosion_equipment(
    game: &mut crate::game::Game<'_>,
    context: &ExplosionHookContext<'_>,
    radius: f32,
    damage_type: &str,
    exploding: Option<u32>,
    attacker: Option<&(u32, Text)>,
    cause: Option<&horizon_client::advancements::EntityView>,
) -> std::ops::ControlFlow<Option<[f64; 3]>> {
    if !horizon_client::health::pluto_protection::enabled()
        && !crate::game::pluto_block_feedback::enabled()
    {
        return std::ops::ControlFlow::Continue(());
    }
    fn original(
        game: &mut crate::game::Game<'_>,
        context: &ExplosionHookContext<'_>,
        radius: f32,
        damage_type: &str,
        exploding: Option<u32>,
        attacker: Option<&(u32, Text)>,
        cause: Option<&horizon_client::advancements::EntityView>,
    ) -> Option<[f64; 3]> {
        let ExplosionHookContext { center, blocks } = *context;
        let diameter = f64::from(radius * 2.0);
        // Blast protection: `EXPLOSION_KNOCKBACK_RESISTANCE` (0.15 per level and
        // piece, so two Blast Protection IV pieces cancel it).
        let resistance = game
            .player
            .as_mut()
            .and_then(|p| p.attribute("minecraft:explosion_knockback_resistance").ok())
            .unwrap_or(0.0);
        let p = game.player.as_ref()?;
        let feet = [p.position.x, p.position.y, p.position.z];
        let dist = ((feet[0] - center[0]).powi(2)
            + (feet[1] - center[1]).powi(2)
            + (feet[2] - center[2]).powi(2))
        .sqrt()
            / diameter;
        if dist > 1.0 || p.health.dead_or_dying() {
            return None;
        }
        // ServerPlayer.onExplosionHit: the blast that hit the player is theirs.
        game.adv.marks.blast = Some((feet, cause.cloned()));
        let w = f64::from(p.dimensions.width) / 2.0;
        let h = f64::from(p.dimensions.height);
        let eye = [
            feet[0],
            feet[1] + f64::from(p.dimensions.eye_height),
            feet[2],
        ];
        let exposure = game.seen_percent(
            center,
            [feet[0] - w, feet[1], feet[2] - w],
            [feet[0] + w, feet[1] + h, feet[2] + w],
        );
        let pow = (1.0 - dist) * f64::from(exposure);
        let amount = ((pow * pow + pow) / 2.0 * 7.0 * diameter + 1.0) as f32;
        let d = [eye[0] - center[0], eye[1] - center[1], eye[2] - center[2]];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let dir = if l < 1.0e-4 {
            [0.0; 3]
        } else {
            [d[0] / l, d[1] / l, d[2] / l]
        };
        let power = pow * (1.0 - resistance);
        // `source.doTeamsAllowDamage(entity)`.
        let hurtable = exploding.is_none_or(|id| {
            horizon_client::scoreboard::teams_allow_damage(game.mob_team(id), game.player_team())
        });
        let blocking = game.player_blocking(Some(center), damage_type, false);
        let raised = blocking.is_some();
        let (creative, spectator) = (game.creative, game.spectator());
        let flying = game.player.as_ref().is_some_and(|p| p.flying);
        let mut outcome = horizon_client::health::BlockOutcome::default();
        let mut pushed = None;
        // The death message names the blast's causing entity. It goes to this
        // player's tracker, the host's or a guest's.
        if let Some((id, name)) = attacker.filter(|_| !creative && !spectator && hurtable) {
            game.note_named_attacker(damage_type, *id, name.clone());
        }
        let equipment = crate::game::pluto_damage_equipment::capture(game);
        if let Some(p) = game.player.as_mut()
            && let Some(id) = game.health_resources.damage.id(damage_type)
        {
            // Creative players are invulnerable to explosions.
            if !creative
                && !spectator
                && hurtable
                && let Ok(o) =
                    horizon_client::health::pluto_protection::hurt_with_optional_equipment(
                        p,
                        &game.health_resources,
                        game.world.health_rules,
                        id,
                        amount,
                        blocking,
                        attacker.is_some(),
                        // The blast's causing entity.
                        attacker.map_or(0, |(id, _)| *id as i32),
                        equipment.as_ref(),
                    )
            {
                outcome = o;
                // dealDefaultKnockback's `indicateDamage` points at the blast
                // (the source position), not the no-direction fallback.
                if o.knockback && o.blocked <= 0.0 {
                    p.health
                        .indicate_damage(center[0] - feet[0], center[2] - feet[2], p.yaw);
                }
            }
        }
        if !spectator {
            let mut push = [dir[0] * power, dir[1] * power, dir[2] * power];
            game.native_explosion_knockback(game.dim.id(), center, blocks, None, &mut push);
            if let Some(p) = game.player.as_mut() {
                p.velocity = Vec3::new(
                    p.velocity.x + push[0],
                    p.velocity.y + push[1],
                    p.velocity.z + push[2],
                );
                if !(creative && flying) {
                    // ServerExplosion.hitPlayers: the packet carries the push even when the hurt
                    // also marks the velocity (its motion packet follows).
                    pushed = Some(push);
                }
            }
        }
        game.combat.forget_noted(outcome.canceled);
        if raised {
            let source = horizon_client::health::HurtSource {
                damage_type,
                causing: attacker.map_or(0, |(id, _)| *id as i32),
                direct: 0,
            };
            crate::game::pluto_block_feedback::dispatch(
                game,
                crate::game::pluto_block_feedback::Event {
                    outcome,
                    attacker: None,
                    projectile: false,
                    source,
                },
            );
            game.after_block(outcome, (None, false), source);
        }
        pushed
    }
    std::ops::ControlFlow::Break(original(
        game,
        context,
        radius,
        damage_type,
        exploding,
        attacker,
        cause,
    ))
}
