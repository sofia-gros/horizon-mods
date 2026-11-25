//! Counterpart of Fabric's `fabric-data-attachment-api-v1`: persistent player/entity data.
//!
//! Fabric: `AttachmentRegistry.create(id, builder)`, then `player.getAttached(type)`, `setAttached(type, value)`, `hasAttached(type)`.
//! Here the value type is any `Serialize + DeserializeOwned` Rust type, stored as JSON (so it is saved with the player by the game's own
//! attachment system). EntityInfo uses the current world's native entity holder. Worlds,
//! chunks and block entities are not exposed here yet. Synced types request tracking-client transport.

use serde::{de::DeserializeOwned, Serialize};

use super::base::Identifier;
use super::player_api::{with_current, ServerPlayer};
use super::entity_events_v1::EntityInfo;

/// A kind of data a player can carry (Fabric: `AttachmentType<A>`).
pub struct AttachmentType<T> {
    id: Identifier,
    default: fn() -> T,
    sync: bool,
}

impl<T> AttachmentType<T> {
    pub fn id(&self) -> &Identifier {
        &self.id
    }
}

/// Fabric: `AttachmentRegistry`.
pub struct AttachmentRegistry;

impl AttachmentRegistry {
    /// Fabric: `AttachmentRegistry.create(id, builder.initializer(default))`.
    pub fn create<T: Serialize + DeserializeOwned>(id: Identifier, default: fn() -> T) -> AttachmentType<T> {
        AttachmentType { id, default, sync: false }
    }
    /// Persistent JSON data also sent by the host to tracking clients. One ID
    /// has one policy: using both synced and unsynced handles refuses access.
    pub fn create_synced<T: Serialize + DeserializeOwned>(id: Identifier, default: fn() -> T) -> AttachmentType<T> {
        AttachmentType { id, default, sync: true }
    }
}

impl ServerPlayer {
    /// Fabric: `hasAttached`. False when nothing is stored, and outside a callback that carries a player.
    pub fn has_attached<T: Serialize + DeserializeOwned>(&self, attachment: &AttachmentType<T>) -> bool {
        self.get_attached_existing(attachment).is_some()
    }

    /// Fabric: `getAttached`: the stored value, or `None` if nothing is stored (or it cannot be read back).
    pub fn get_attached_existing<T: Serialize + DeserializeOwned>(&self, attachment: &AttachmentType<T>) -> Option<T> {
        let json = with_current(None, |ops| ops.attachment_get_with_policy(self.uuid, &attachment.id.to_string(), attachment.sync))?;
        serde_json::from_str(&json).ok()
    }

    /// Fabric: `getAttachedOrCreate`/`getAttachedOrElse`: the stored value, or the type's default (not stored).
    pub fn get_attached<T: Serialize + DeserializeOwned>(&self, attachment: &AttachmentType<T>) -> T {
        self.get_attached_existing(attachment).unwrap_or_else(attachment.default)
    }

    /// Fabric: `setAttached`. False when the value cannot be stored or this is called outside a callback.
    pub fn set_attached<T: Serialize + DeserializeOwned>(&self, attachment: &AttachmentType<T>, value: T) -> bool {
        match serde_json::to_string(&value) {
            Ok(json) => with_current(false, |ops| ops.attachment_set_with_policy(self.uuid, &attachment.id.to_string(), &json, attachment.sync)),
            Err(_) => false,
        }
    }
}

impl EntityInfo {
    /// Whether this callback's entity has decodable data. Outside a host scope
    /// or for a missing/wrong-dimension native holder, returns false.
    pub fn has_attached<T: Serialize + DeserializeOwned>(&self, attachment: &AttachmentType<T>) -> bool {
        self.get_attached_existing(attachment).is_some()
    }
    /// Read persistent entity data without inserting a default. Numeric IDs are
    /// scoped to this world/callback, not an enduring entity reference.
    pub fn get_attached_existing<T: Serialize + DeserializeOwned>(&self, attachment: &AttachmentType<T>) -> Option<T> {
        let json = with_current(None, |ops| ops.entity_attachment_get(self.id, &self.dimension, &attachment.id.to_string(), attachment.sync))?;
        serde_json::from_str(&json).ok()
    }
    /// Read existing data, otherwise return an unstored default.
    pub fn get_attached<T: Serialize + DeserializeOwned>(&self, attachment: &AttachmentType<T>) -> T {
        self.get_attached_existing(attachment).unwrap_or_else(attachment.default)
    }
    /// Store data on the current native entity holder, preserving native save
    /// and tracking-client sync behavior. False when unsupported/unavailable.
    pub fn set_attached<T: Serialize + DeserializeOwned>(&self, attachment: &AttachmentType<T>, value: T) -> bool {
        match serde_json::to_string(&value) {
            Ok(json) => with_current(false, |ops| ops.entity_attachment_set(self.id, &self.dimension, &attachment.id.to_string(), &json, attachment.sync)),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::player_api::{with_ops, PlayerOps};
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Store(HashMap<(u128, String), String>);

    impl PlayerOps for Store {
        fn tell(&mut self, _: u128, _: &str) -> bool {
            true
        }
        fn give(&mut self, _: u128, _: &str, count: u32) -> u32 {
            count
        }
        fn attachment_get(&mut self, player: u128, id: &str) -> Option<String> {
            self.0.get(&(player, id.to_owned())).cloned()
        }
        fn attachment_set(&mut self, player: u128, id: &str, json: &str) -> bool {
            self.0.insert((player, id.to_owned()), json.to_owned());
            true
        }
        fn count_item(&mut self, _: u128, _: &str) -> u32 {
            0
        }
        fn take_item(&mut self, _: u128, _: &str, _: u32) -> u32 {
            0
        }
        fn empty_slots(&mut self, _: u128) -> u32 {
            0
        }
        fn language(&mut self, _: u128) -> Option<String> {
            None
        }
        fn add_effect(&mut self, _: u128, _: &crate::pure::player_api::EffectInstance) -> bool {
            false
        }
        fn remove_effect(&mut self, _: u128, _: &str) -> bool {
            false
        }
        fn effects(&mut self, _: u128) -> Option<Vec<crate::pure::player_api::EffectInstance>> {
            None
        }
        fn food(&mut self, _: u128) -> Option<crate::pure::player_api::FoodStatus> {
            None
        }
    }

    #[test]
    fn entity_data_is_per_world_and_holder_policy_is_immutable_and_snapshots_round_trip() {
        use crate::pure::player_api::MemoryPlayerOps;
        let kind = AttachmentRegistry::create_synced(Identifier::parse("test:metal").unwrap(), || false);
        let entity = |id, dimension: &str| EntityInfo { id, kind: "test:slime".into(), dimension: dimension.into() };
        let a = entity(1, "test:a"); let b = entity(2, "test:a"); let other_world = entity(1, "test:b");
        let mut game = MemoryPlayerOps::default();
        with_ops(&mut game, || {
            assert!(!a.has_attached(&kind));
            assert!(a.set_attached(&kind, true));
            assert_eq!(a.get_attached_existing(&kind), Some(true));
            assert_eq!(b.get_attached_existing(&kind), None);
            assert_eq!(other_world.get_attached_existing(&kind), None);
            let conflicting = AttachmentRegistry::create(Identifier::parse("test:metal").unwrap(), || false);
            assert!(!a.set_attached(&conflicting, false));
            assert_eq!(a.get_attached_existing(&kind), Some(true));
        });
        assert!(!a.set_attached(&kind, false));
        let saved = game.entity_attachments[&("test:a".into(), 1, "test:metal".into())].clone();
        assert!(serde_json::from_str::<bool>(&saved).unwrap());
        let mut reopened = MemoryPlayerOps::default();
        reopened.entity_attachments.insert(("test:a".into(), 1, "test:metal".into()), saved);
        with_ops(&mut reopened, || assert_eq!(a.get_attached_existing(&kind), Some(true)));
    }

    fn counter() -> AttachmentType<i32> {
        AttachmentRegistry::create(Identifier::new("test", "counter").unwrap(), || 0)
    }

    #[test]
    fn a_value_is_default_until_set_then_comes_back_and_is_kept_per_player() {
        let mut store = Store::default();
        let (a, b) = (ServerPlayer { uuid: 1, name: "a".into() }, ServerPlayer { uuid: 2, name: "b".into() });
        with_ops(&mut store, || {
            assert!(!a.has_attached(&counter()));
            assert_eq!(a.get_attached(&counter()), 0);
            assert!(a.set_attached(&counter(), 5));
            assert!(a.has_attached(&counter()));
            assert_eq!(a.get_attached(&counter()), 5);
            assert_eq!(b.get_attached(&counter()), 0, "another player has its own value");
        });
    }

    #[test]
    fn structured_values_round_trip_and_nothing_is_stored_outside_a_callback() {
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Book {
            given: bool,
            times: u32,
        }
        let kind = AttachmentRegistry::create(Identifier::new("test", "book").unwrap(), || Book { given: false, times: 0 });
        let player = ServerPlayer { uuid: 9, name: "p".into() };
        assert!(!player.set_attached(&kind, Book { given: true, times: 1 }), "outside a scope nothing can be stored");
        let mut store = Store::default();
        with_ops(&mut store, || {
            assert!(player.set_attached(&kind, Book { given: true, times: 2 }));
            assert_eq!(player.get_attached(&kind), Book { given: true, times: 2 });
        });
    }
}
