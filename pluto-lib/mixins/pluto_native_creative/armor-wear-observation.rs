// Private authoritative bridge. An observation records consumption-time facts,
// not per-piece native wear decisions or an authorization to mutate equipment.
use super::{Game, pluto_equipment_view::Snapshot};
use horizon_client::pluto_inventory::{armor_wear as api, equipment::Actor};
use horizon_world::damage_source::DamageTypeId;

// These observed fields are a synchronous scope check, not a world generation
// or retained transaction identity; identical-tuple replacement is undetectable.
#[derive(PartialEq, Eq)]
struct Scope {
    entity: i32,
    dimension: String,
    world_root: std::path::PathBuf,
    guest: Option<u32>,
    guest_uuid: Option<u128>,
    guest_entity: Option<u32>,
    creative: bool,
    spectator: bool,
}
impl Scope {
    fn capture(game: &Game<'_>) -> Option<Self> {
        if game.remote.is_some() {
            return None;
        }
        let player = game.player.as_ref()?;
        if !horizon_client::health::pluto_protection::equipment_allowed(player) {
            return None;
        }
        Some(Self {
            entity: player.entity_id,
            dimension: game.dim.id().to_owned(),
            world_root: game.world_root.clone(),
            guest: game.lan_acting,
            guest_uuid: game.lan_acting_uuid,
            guest_entity: game.lan_acting_entity,
            creative: game.creative,
            spectator: game.spectator(),
        })
    }
    fn actor(&self) -> Actor<'_> {
        Actor {
            entity: self.entity,
            dimension: &self.dimension,
            creative: self.creative,
            spectator: self.spectator,
        }
    }
}

pub(crate) struct Observation {
    scope: Scope,
    source: Option<String>,
    requested_damage: f32,
    before: Snapshot,
}
/// Called after earlier HEAD handlers, immediately before the unchanged native
/// body. No allocation/conversion/native policy changes when no observer exists.
pub(crate) fn begin(
    game: &Game<'_>,
    source: DamageTypeId,
    requested_damage: f32,
) -> Option<Observation> {
    if !api::enabled() {
        return None;
    }
    let scope = Scope::capture(game)?;
    Some(Observation {
        scope,
        source: game
            .health_resources
            .damage
            .definition(source)
            .map(|d| d.name.clone()),
        requested_damage,
        before: Snapshot::capture(game),
    })
}
impl Observation {
    /// Native wear, destroy-item feedback and apply_equipment have already run.
    /// A changed or no-longer-authoritative actor/world suppresses observation;
    /// native operations are never retried, rolled back or amended here.
    pub(crate) fn finish(self, game: &Game<'_>) {
        if Scope::capture(game).as_ref() != Some(&self.scope) {
            return;
        }
        let after = Snapshot::capture(game);
        api::dispatch(&api::Request {
            actor: self.scope.actor(),
            source_kind: self.source.as_deref(),
            requested_damage: self.requested_damage,
            before: &self.before,
            after: &after,
        });
    }
}
