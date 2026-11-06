// UNAPPLIED / UNEXECUTED fixture helper for a separate applied-v15 Game overlay.
// NOT a continuation validator, installed hook, public API or runtime test result.
#[allow(dead_code)]
mod pluto_guest_continuation_observer {
    use super::{Game, commands::Target, lan::LanHost};
    use horizon_client::mod_api::{EntityRef, NativeWorldEpoch};

    #[derive(Debug, PartialEq)]
    struct BodyFacts {
        id: i32,
        position: [f64; 3],
        velocity: [f64; 3],
        fall_distance: f64,
        on_ground: bool,
    }

    #[derive(Debug, PartialEq)]
    struct OwnerFacts {
        acting: Option<u32>,
        uuid: Option<u128>,
        entity: Option<u32>,
        dimension: String,
        body: Option<BodyFacts>,
    }

    #[derive(Debug)]
    struct Sample {
        owner: OwnerFacts,
        ready_epoch: Option<NativeWorldEpoch>,
        handle: Option<EntityRef>,
        pending_teleport_notes: usize,
    }

    #[derive(Debug)]
    struct During {
        before: Sample,
        away: Option<Sample>,
        returned: Option<Sample>,
        failure: Option<&'static str>,
    }

    #[derive(Debug)]
    struct Observation {
        outer_before: OwnerFacts,
        // None means actual native as_guest did not enter. Never treat as pass.
        during: Option<During>,
        // Captured AFTER leave_guest and its native notification flush return.
        outer_after: OwnerFacts,
        roster_position_after: Option<[f64; 3]>,
    }

    fn owner(game: &Game<'_>) -> OwnerFacts {
        OwnerFacts {
            acting: game.lan_acting,
            uuid: game.lan_acting_uuid,
            entity: game.lan_acting_entity,
            dimension: game.dim.id().to_owned(),
            body: game.player.as_ref().map(|p| BodyFacts {
                id: p.entity_id,
                position: [p.position.x, p.position.y, p.position.z],
                velocity: [p.velocity.x, p.velocity.y, p.velocity.z],
                fall_distance: p.fall_distance,
                on_ground: p.ground.on_ground,
            }),
        }
    }

    fn sample(game: &Game<'_>) -> Sample {
        let handle = game.player.as_ref()
            .and_then(|p| u32::try_from(p.entity_id).ok())
            .and_then(|id| game.native_entity(id));
        Sample {
            owner: owner(game),
            ready_epoch: super::native_lifecycle::pluto_lifecycle_readiness::ready_server_epoch(
                &game.native_lifecycle,
            ),
            handle,
            pending_teleport_notes: game.teleport_notes.len(),
        }
    }

    // Requires a future real-resource harness with an actual connected, loaded
    // guest. It supplies Game/host; this helper constructs neither nor repairs
    // lifecycle/roster state. The initial readonly checks select a bounded
    // fixture case, not permission for arbitrary entity operations.
    fn observe_round_trip(
        game: &mut Game<'_>, host: &mut LanHost, conn: u32, away: [f64; 3],
    ) -> Result<Observation, &'static str> {
        if game.remote.is_some() || game.lan.is_some() || game.lan_acting.is_some() {
            return Err("requires actual detached-host, non-reentrant server guest path");
        }
        if super::native_lifecycle::pluto_lifecycle_readiness::ready_server_epoch(
            &game.native_lifecycle,
        ).is_none() {
            return Err("server is not Ready; an epoch alone is insufficient");
        }
        let guests: Vec<_> = host.hub.guests.iter().filter(|g| g.conn == conn).collect();
        if guests.len() != 1 {
            return Err("requested guest is absent or ambiguous");
        }
        let guest = guests[0];
        if !guest.placed || !guest.loaded || guest.dead || guest.motion.is_none()
            || guest.vehicle.is_some() || guest.dimension != game.dim.id()
        {
            return Err("fixture requires one loaded, living, unmounted same-level body");
        }
        let (id, uuid, origin) = (guest.entity_id, guest.uuid, guest.position);
        if !origin.iter().chain(away.iter()).all(|v| v.is_finite()) || origin == away {
            return Err("finite distinct round-trip endpoints required");
        }
        let outer_before = owner(game);
        let during = game.as_guest_with_host(host, conn, |active, actual_host| {
            let before = sample(active);
            let roster = actual_host.hub.guests.iter().find(|g| g.conn == conn);
            let entered = roster.is_some_and(|g| g.entity_id == id && g.uuid == uuid)
                && active.lan_acting == Some(conn)
                && active.lan_acting_uuid == Some(uuid)
                && active.player.as_ref().is_some_and(|p| p.entity_id == id);
            if !entered {
                return During { before, away: None, returned: None,
                    failure: Some("native entry did not expose requested roster/body") };
            }
            active.teleport_to_in_level(Target::Player, away, None, false);
            let moved = sample(active);
            // Admission to the second explicit fixture operation only, NOT a
            // generic saved-frame validator. These fields cannot detect an
            // equal-ID body replacement or an intervening out-and-back action.
            let same_fixture_case = moved.owner.acting == before.owner.acting
                && moved.owner.uuid == before.owner.uuid
                && moved.owner.entity == before.owner.entity
                && moved.owner.dimension == before.owner.dimension
                && moved.ready_epoch == before.ready_epoch
                && moved.ready_epoch.is_some()
                && moved.owner.body.as_ref().is_some_and(|p| p.id == id && p.position == away);
            if !same_fixture_case {
                return During { before, away: Some(moved), returned: None,
                    failure: Some("actor, dimension, Ready phase or expected body position changed") };
            }
            // An intentional second native operation, NOT rollback: velocity,
            // fall state, notes, packets and callbacks are not undone.
            active.teleport_to_in_level(Target::Player, origin, None, false);
            During { before, away: Some(moved), returned: Some(sample(active)), failure: None }
        });
        Ok(Observation {
            outer_before,
            during,
            outer_after: owner(game),
            roster_position_after: host.hub.guests.iter().find(|g| g.conn == conn).map(|g| g.position),
        })
    }
}
