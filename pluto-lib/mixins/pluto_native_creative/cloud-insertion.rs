// Native publication boundary. A requested owner is never selected implicitly.
use super::{Game, commands::Target, native_lifecycle::pluto_lifecycle_readiness,
    potions::pluto_cloud_factory};
use horizon_client::{mod_api::{EntityRef, LevelRef}, pluto_cloud::{Attribution, Validated}};

/// The source chosen by the caller. Entity lookup proves presence, not causation
/// or permission to impersonate it. Consumers must prove their action context.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::game) enum Owner {
    Unowned,
    Entity(EntityRef),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::game) enum Error {
    NotAuthoritative,
    NotReady,
    StaleOrForeignLevel,
    LevelNotInPlace,
    ActorUnavailable,
    ActorOutsideLevel,
    UnsupportedActor,
    AttributionMismatch,
    CenterNotLoaded,
    EntityIdExhausted,
    Capacity,
    ActionContextChanged,
}

/// An observed attached server-player frame, not a grant to cause an action.
/// The caller must be handling that player's action in this invocation. This
/// value cannot be constructed from numeric IDs by a consumer. A later use is
/// revalidated; it neither parks a Game borrow nor keeps a roster alive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::game) struct AttachedPlayerFrame {
    level: LevelRef,
    entity: EntityRef,
    connection: u32,
    uuid: u128,
}
impl AttachedPlayerFrame {
    pub(in crate::game) fn attribution(&self) -> Attribution {
        Attribution::Player { guest_connection: Some(self.connection) }
    }
}

/// Observe only a Ready, attached, placed acting-player roster/motion pair.
/// A detached `as_guest` frame is intentionally unavailable. No current-player
/// fallback, owner=None, synthetic handle, guest swap or world change is used.
pub(in crate::game) fn attached_player_frame(game: &Game<'_>) -> Result<AttachedPlayerFrame, Error> {
    if game.remote.is_some() || !game.native_authoritative() { return Err(Error::NotAuthoritative); }
    let epoch = pluto_lifecycle_readiness::ready_server_epoch(&game.native_lifecycle)
        .ok_or(Error::NotReady)?;
    let level = game.native_level(game.dim.id()).ok_or(Error::StaleOrForeignLevel)?;
    if level.epoch() != epoch || game.native_level_height(level).is_none() {
        return Err(Error::StaleOrForeignLevel);
    }
    let connection = game.lan_acting.ok_or(Error::ActorUnavailable)?;
    let guest = game.command_guest(connection).ok_or(Error::ActorUnavailable)?;
    let motion = game.player.as_ref().ok_or(Error::ActorUnavailable)?;
    let id = u32::try_from(motion.entity_id).map_err(|_| Error::ActorUnavailable)?;
    if !guest.placed || !guest.loaded || guest.dead || guest.dimension != game.dim.id()
        || guest.entity_id != motion.entity_id
        || game.lan_acting_entity != Some(id) || game.lan_acting_uuid != Some(guest.uuid) {
        return Err(Error::ActorUnavailable);
    }
    let entity = game.native_entity(id).ok_or(Error::ActorUnavailable)?;
    if !matches!(game.native_entity_target(entity), Some(Target::Player))
        || game.native_entity_level(entity) != Some(level) {
        return Err(Error::ActorOutsideLevel);
    }
    let frame = AttachedPlayerFrame { level, entity, connection, uuid: guest.uuid };
    require_attribution(resolve_owner(game, level, Owner::Entity(entity))?, frame.attribution())?;
    Ok(frame)
}

/// Recheck the same current frame after any caller-controlled work. Failure
/// does not roll back earlier caller/world effects and must not charge another
/// actor. Matching this observation does not prove held-item or action identity.
pub(in crate::game) fn require_attached_player_frame(game: &Game<'_>, expected: &AttachedPlayerFrame)
    -> Result<(), Error> {
    if attached_player_frame(game)? == *expected { Ok(()) }
    else { Err(Error::ActionContextChanged) }
}

/// Narrow convenience over the existing explicit insertion transaction. It
/// adds a same-frame check; it does not relax any original insertion rejection.
pub(in crate::game) fn insert_attached_player_frame(game: &mut Game<'_>, frame: &AttachedPlayerFrame,
    request: Validated) -> Result<EntityRef, Error> {
    require_attached_player_frame(game, frame)?;
    insert(game, frame.level, Owner::Entity(frame.entity), request)
}

/// Resolve only through official current-world/roster lookups. In particular,
/// an active guest with its LAN hub taken out is unavailable, not guessed from
/// numeric acting fields or read-only equipment Actor facts.
fn resolve_owner(game: &Game<'_>, level: LevelRef, owner: Owner) -> Result<Attribution, Error> {
    let Owner::Entity(entity) = owner else { return Ok(Attribution::Unowned); };
    let target = game.native_entity_target(entity).ok_or(Error::ActorUnavailable)?;
    if game.native_entity_level(entity) != Some(level) { return Err(Error::ActorOutsideLevel); }
    match target {
        Target::Mob(id) => Ok(Attribution::Mob { entity_id: id }),
        Target::Guest(conn) => {
            let guest = game.command_guest(conn).ok_or(Error::ActorUnavailable)?;
            if !guest.placed || u32::try_from(guest.entity_id).ok() != Some(entity.id()) {
                return Err(Error::ActorUnavailable);
            }
            Ok(Attribution::Player { guest_connection: Some(conn) })
        }
        Target::Player => {
            let conn = game.lan_acting.ok_or(Error::ActorUnavailable)?;
            let guest = game.command_guest(conn).ok_or(Error::ActorUnavailable)?;
            let motion = game.player.as_ref().ok_or(Error::ActorUnavailable)?;
            if !guest.placed || u32::try_from(guest.entity_id).ok() != Some(entity.id())
                || motion.entity_id != guest.entity_id
                || game.lan_acting_entity != Some(entity.id())
                || game.lan_acting_uuid != Some(guest.uuid) {
                return Err(Error::ActorUnavailable);
            }
            Ok(Attribution::Player { guest_connection: Some(conn) })
        }
        _ => Err(Error::UnsupportedActor),
    }
}

fn require_attribution(resolved: Attribution, requested: Attribution) -> Result<(), Error> {
    if resolved == requested { Ok(()) } else { Err(Error::AttributionMismatch) }
}

fn require_entity_successor(last: u32) -> Result<(), Error> {
    // Native entity lookup/network IDs use i32; never publish an unresolvable ID.
    match last.checked_add(1) {
        Some(next) if i32::try_from(next).is_ok() => Ok(()),
        _ => Err(Error::EntityIdExhausted),
    }
}

/// Insert into the active Ready authoritative level with explicit attribution.
/// Does not switch levels, select an owner, apply effects, or flush LAN events.
/// All recoverable failures precede allocation/publication; no rollback follows
/// the native take-ID / assign-ID / vector-push commit sequence.
#[allow(dead_code)]
pub(in crate::game) fn insert(game: &mut Game<'_>, level: LevelRef, owner: Owner,
    request: Validated) -> Result<EntityRef, Error> {
    if game.remote.is_some() || !game.native_authoritative() { return Err(Error::NotAuthoritative); }
    let epoch = pluto_lifecycle_readiness::ready_server_epoch(&game.native_lifecycle)
        .ok_or(Error::NotReady)?;
    if level.epoch() != epoch || game.native_level(level.dimension()) != Some(level)
        || game.native_level_height(level).is_none() {
        return Err(Error::StaleOrForeignLevel);
    }
    if level.dimension() != game.dim.id() { return Err(Error::LevelNotInPlace); }
    let resolved = resolve_owner(game, level, owner)?;
    require_attribution(resolved, request.descriptor().attribution)?;
    let [x, _, z] = request.descriptor().position.map(|n| n.floor() as i32);
    if !game.world.loaded(x, z) { return Err(Error::CenterNotLoaded); }
    require_entity_successor(game.next_entity)?;
    let mut native = pluto_cloud_factory::construct(request).into_native();
    game.breath_clouds.try_reserve(1).map_err(|_| Error::Capacity)?;
    let id = game.take_entity_id();
    native.id = id;
    game.breath_clouds.push(native);
    Ok(EntityRef::new(epoch, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attached_frame_identity_includes_epoch_level_entity_connection_and_uuid() {
        use horizon_client::mod_api::NativeWorldEpoch;
        let epoch = NativeWorldEpoch::new(7);
        let frame = AttachedPlayerFrame { level: LevelRef::new(epoch, "minecraft:overworld"),
            entity: EntityRef::new(epoch, 23), connection: 4, uuid: 100 };
        assert_eq!(frame, frame);
        for changed in [
            AttachedPlayerFrame { level: LevelRef::new(NativeWorldEpoch::new(8), "minecraft:overworld"), ..frame },
            AttachedPlayerFrame { level: LevelRef::new(epoch, "minecraft:the_nether"), ..frame },
            AttachedPlayerFrame { entity: EntityRef::new(epoch, 24), ..frame },
            AttachedPlayerFrame { entity: EntityRef::new(NativeWorldEpoch::new(8), 23), ..frame },
            AttachedPlayerFrame { connection: 5, ..frame },
            AttachedPlayerFrame { uuid: 101, ..frame },
        ] { assert_ne!(frame, changed); }
    }

    #[test]
    fn attached_frame_requests_only_its_explicit_player_connection() {
        use horizon_client::mod_api::NativeWorldEpoch;
        let epoch=NativeWorldEpoch::new(1);
        let frame=AttachedPlayerFrame { level: LevelRef::new(epoch,"minecraft:overworld"),
            entity: EntityRef::new(epoch,5),connection:9,uuid:22 };
        assert_eq!(frame.attribution(),Attribution::Player { guest_connection:Some(9) });
        assert_ne!(frame.attribution(),Attribution::Unowned);
        assert_ne!(frame.attribution(),Attribution::Player { guest_connection:None });
    }

    #[test]
    fn requested_owner_must_match_including_guest_and_mob_identity() {
        let values = [Attribution::Unowned,
            Attribution::Player { guest_connection: None },
            Attribution::Player { guest_connection: Some(3) },
            Attribution::Player { guest_connection: Some(4) },
            Attribution::Mob { entity_id: 3 }, Attribution::Mob { entity_id: 4 }];
        for resolved in values {
            for requested in values {
                assert_eq!(require_attribution(resolved, requested),
                    if resolved == requested { Ok(()) } else { Err(Error::AttributionMismatch) });
            }
        }
    }

    #[test]
    fn no_allocated_id_can_overflow_or_escape_native_lookup_range() {
        assert_eq!(require_entity_successor(0), Ok(()));
        assert_eq!(require_entity_successor(i32::MAX as u32 - 1), Ok(()));
        for last in [i32::MAX as u32, i32::MAX as u32 + 1, u32::MAX] {
            assert_eq!(require_entity_successor(last), Err(Error::EntityIdExhausted));
        }
    }
}
