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

// BEGIN UNAPPLIED NATIVE ITEM FRAME CANDIDATE
// This observation is not an authority capability. Only the source-pinned
// completed native item-use invocation may call this entry. Complete installed
// composition must preserve that caller; matching fields alone do not prove it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeItemActor {
    Guest { connection: u32, uuid: u128, entity_id: u32 },
    PlayerWithoutGuest { body_entity_id: i32 },
}
impl NativeItemActor {
    fn attribution(self) -> Attribution {
        Attribution::Player { guest_connection: match self {
            Self::Guest { connection, .. } => Some(connection),
            Self::PlayerWithoutGuest { .. } => None,
        } }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::game) struct NativeItemFrame {
    level: LevelRef,
    actor: NativeItemActor,
    phase: u8,
    hand: u8,
    held_slot: usize,
    creative: bool,
}
impl NativeItemFrame {
    pub(in crate::game) fn attribution(&self) -> Attribution { self.actor.attribution() }
}

// Classifies a previously admitted native frame; it never manufactures a
// native entity handle or resolves numeric fields as an authorization check.
fn item_actor(body: i32, connection: Option<u32>, uuid: Option<u128>, entity: Option<u32>,
    phase: u8) -> Result<NativeItemActor, Error> {
    if phase > 2 { return Err(Error::UnsupportedActor); }
    match (connection, uuid, entity) {
        (Some(connection), Some(uuid), Some(entity_id)) if phase != 0
            && entity_id == body as u32 =>
            Ok(NativeItemActor::Guest { connection, uuid, entity_id }),
        (None, None, None) => Ok(NativeItemActor::PlayerWithoutGuest { body_entity_id: body }),
        _ => Err(Error::ActorUnavailable),
    }
}

/// Only from the current native use_block_transformer held-item action.
/// Guest packet admission already checked the real host/guest before as_guest;
/// the detached invocation exposes its established tuple, not an attached
/// roster. Local/fake use_item also supplies Who::Player, with guest=None.
/// Neither branch promises a permanent entity/UUID for a fake player.
pub(in crate::game) fn native_item_frame(game: &Game<'_>) -> Result<NativeItemFrame, Error> {
    if game.remote.is_some() || !game.native_authoritative() { return Err(Error::NotAuthoritative); }
    let epoch = pluto_lifecycle_readiness::ready_server_epoch(&game.native_lifecycle)
        .ok_or(Error::NotReady)?;
    let level = game.native_level(game.dim.id()).ok_or(Error::StaleOrForeignLevel)?;
    if level.epoch() != epoch || game.native_level_height(level).is_none() {
        return Err(Error::StaleOrForeignLevel);
    }
    if !matches!(game.sculk_actor(), Some(horizon_client_core::who::Who::Player)) {
        return Err(Error::UnsupportedActor);
    }
    let motion = game.player.as_ref().ok_or(Error::ActorUnavailable)?;
    let phase = match game.guest_use {
        None => 0,
        Some(super::GuestUse::OnBlock(super::Hand::Main)) => 1,
        Some(super::GuestUse::OnBlock(super::Hand::Off)) => 2,
        _ => return Err(Error::UnsupportedActor),
    };
    let hand = match game.hand { super::Hand::Main => 1, super::Hand::Off => 2 };
    if phase != 0 && phase != hand { return Err(Error::ActionContextChanged); }
    let actor = item_actor(motion.entity_id, game.lan_acting, game.lan_acting_uuid,
        game.lan_acting_entity, phase)?;
    Ok(NativeItemFrame { level, actor, phase, hand,
        held_slot: game.held_slot(), creative: game.creative })
}

/// Same-invocation continuation check, not an ABA-proof lease. Complete-plan
/// review must exclude arbitrary actor re-entry/replacement across callbacks.
/// The consumer separately checks the held stack before charging it.
pub(in crate::game) fn require_native_item_frame(game: &Game<'_>, expected: &NativeItemFrame)
    -> Result<(), Error> {
    if native_item_frame(game)? == *expected { Ok(()) }
    else { Err(Error::ActionContextChanged) }
}

/// Native item-action entry, separate from the unchanged explicit EntityRef
/// resolver below. Its authority comes from the exact caller contract, not the
/// snapshot. The requested attribution must retain the native Player/guest pair.
/// All recoverable errors precede the same native allocation/publication tail.
pub(in crate::game) fn insert_native_item_frame(game: &mut Game<'_>, frame: &NativeItemFrame,
    request: Validated) -> Result<EntityRef, Error> {
    require_native_item_frame(game, frame)?;
    require_attribution(frame.attribution(), request.descriptor().attribution)?;
    let epoch = frame.level.epoch();
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
// END UNAPPLIED NATIVE ITEM FRAME CANDIDATE

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

    // These tests exercise proposed classification/value rules, not real Game dispatch.
    #[test]
    fn native_item_guest_keeps_native_player_connection() {
        let actor = item_actor(23, Some(4), Some(100), Some(23), 1).unwrap();
        assert_eq!(actor, NativeItemActor::Guest { connection:4, uuid:100, entity_id:23 });
        assert_eq!(actor.attribution(), Attribution::Player { guest_connection:Some(4) });
        assert!(item_actor(23, Some(4), Some(100), Some(23), 2).is_ok());
        // Match native enter_guest: its signed body ID is stored as u32 bits.
        assert!(item_actor(-1, Some(4), Some(100), Some(u32::MAX), 1).is_ok());
    }

    #[test]
    fn native_item_local_and_fake_keep_player_without_guest_not_unowned() {
        for body in [-1, 0, 23] {
            for phase in [0, 1, 2] {
                let actor = item_actor(body,None,None,None,phase).unwrap();
                assert_eq!(actor,NativeItemActor::PlayerWithoutGuest { body_entity_id:body });
                assert_eq!(actor.attribution(),Attribution::Player { guest_connection:None });
                assert_ne!(actor.attribution(),Attribution::Unowned);
            }
        }
    }

    #[test]
    fn native_item_partial_guest_wrong_body_and_other_phase_fail_closed() {
        for mask in 1..7 {
            assert!(item_actor(23, (mask&1!=0).then_some(4), (mask&2!=0).then_some(100),
                (mask&4!=0).then_some(23), 1).is_err());
        }
        for (body,entity,phase) in [(24,23,1),(-1,23,1),(23,23,0),(23,23,3)] {
            assert!(item_actor(body,Some(4),Some(100),Some(entity),phase).is_err());
        }
        assert!(item_actor(0,None,None,None,3).is_err());
    }

    #[test]
    fn native_item_snapshot_detects_observed_frame_hand_slot_and_mode_changes() {
        use horizon_client::mod_api::NativeWorldEpoch;
        let frame=NativeItemFrame { level:LevelRef::new(NativeWorldEpoch::new(7),"minecraft:overworld"),
            actor:NativeItemActor::Guest { connection:4,uuid:100,entity_id:23 },phase:1,hand:1,
            held_slot:2,creative:false };
        for changed in [
            NativeItemFrame { level:LevelRef::new(NativeWorldEpoch::new(8),"minecraft:overworld"),..frame },
            NativeItemFrame { level:LevelRef::new(NativeWorldEpoch::new(7),"minecraft:the_nether"),..frame },
            NativeItemFrame { actor:NativeItemActor::Guest { connection:5,uuid:100,entity_id:23 },..frame },
            NativeItemFrame { actor:NativeItemActor::Guest { connection:4,uuid:101,entity_id:23 },..frame },
            NativeItemFrame { actor:NativeItemActor::Guest { connection:4,uuid:100,entity_id:24 },..frame },
            NativeItemFrame { actor:NativeItemActor::PlayerWithoutGuest { body_entity_id:23 },..frame },
            NativeItemFrame { phase:2,..frame },NativeItemFrame { hand:2,..frame },
            NativeItemFrame { held_slot:3,..frame },NativeItemFrame { creative:true,..frame },
        ] { assert_ne!(frame,changed); }
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
