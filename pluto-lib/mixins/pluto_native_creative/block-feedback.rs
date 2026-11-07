// Private native bridge. Consumer callbacks live only in the host-independent
// blocking API and never receive Game, native ItemStack or a mutation capability.
use super::Game;
use horizon_client::{
    health::{BlockOutcome, HurtSource},
    pluto_inventory::blocking as api,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Event<'a> {
    pub outcome: BlockOutcome,
    pub attacker: Option<u32>,
    pub projectile: bool,
    pub source: HurtSource<'a>,
}

pub(crate) fn enabled() -> bool {
    api::enabled()
}

/// The source-guarded caller supplies the final native facts at its exact
/// pre-after_block boundary. No read conversion occurs without consumers.
pub(crate) fn dispatch(game: &Game<'_>, event: Event<'_>) {
    if !api::enabled() || game.remote.is_some() {
        return;
    }
    let Some(player) = game
        .player
        .as_ref()
        .filter(|player| horizon_client::health::pluto_protection::equipment_allowed(player))
    else {
        return;
    };
    let equipment = super::pluto_equipment_view::Snapshot::capture(game);
    let outcome = event.outcome;
    api::dispatch(&api::Request {
        event: api::Event {
            outcome: api::Outcome {
                hurt: outcome.hurt,
                blocked: outcome.blocked,
                fully_blocked: outcome.fully_blocked,
                sound: outcome.sound,
                knockback: outcome.knockback,
                canceled: outcome.canceled,
            },
            source: api::Source {
                kind: event.source.damage_type,
                causing: event.source.causing,
                direct: event.source.direct,
            },
            attacker: event.attacker,
            projectile: event.projectile,
        },
        actor: api::Actor {
            entity: player.entity_id,
            dimension: game.dim.id(),
            creative: game.creative,
            spectator: game.spectator(),
        },
        equipment: &equipment,
    });
}
