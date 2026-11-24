//! The guest-client attribute sync and the double-jump control packet (A15).
//!
//! A guest on a LAN server has no server tick of its own: the PLAYER_TICK that computes a
//! player's custom attribute values and armor-stat totals runs on the server, so the guest's
//! client would read the declared defaults for everything its own movement needs (use-item
//! speed, the jump-press gating, the armor-stat terms). The server tick therefore publishes
//! a player's changed values over `tconstruct:attributes/v1` (a small packet, only the
//! values that changed since the last send), and the guest client keeps the synced copy for
//! the readers that run on it. The local player of an integrated server never receives one
//! (the transport does not loop back), so a send that keeps failing is dropped after five
//! server ticks rather than queued forever.
//!
//! The double jump runs the other way. A guest client cannot resolve its own jump press
//! (there is no server tick on it to run the pending jump through), so on a fresh jump
//! press with jumps left it sends an empty `tconstruct:double_jump` packet instead; the
//! server queues the sender and its next tick resolves the request through the pending
//! double jump the tick block already owns, with the server's values and jumps-taken state.
//! The owner of an integrated server resolves the jump locally, as today.

use crate::custom_attributes;
use pluto_lib::{ClientPlayConnectionEvents, ClientPlayNetworking, Identifier, ServerLifecycleEvents, ServerPlayConnectionEvents, ServerPlayNetworking, ServerTickEvents};
use std::collections::{BTreeMap, HashSet};
use std::sync::{LazyLock, Mutex};

/// The server-to-client attribute sync channel: the changed values of a player's custom
/// attributes and armor-stat totals, published on their tick.
const ATTRIBUTES_CHANNEL: &str = "tconstruct:attributes/v1";
/// The client-to-server double-jump channel: an empty payload, sent by a guest client whose
/// jump press the server must run (the guest has no server tick of its own to resolve it on).
const DOUBLE_JUMP_CHANNEL: &str = "tconstruct:double_jump";

const VERSION: u8 = 1;
/// A queued packet is dropped after this many failed server ticks: a local player never
/// receives one (the transport does not loop back), so the queue must not grow forever.
const MAX_FAILED_TICKS: u32 = 5;

fn channel(name: &str) -> Identifier { Identifier::parse(name).expect("attribute sync channel") }

/// The payload: version, entity id, then the changed attribute values as (declared index,
/// value) and the changed armor-stat totals as (key index, value), each value a little-endian
/// f64. At most 12 attributes and 4 armor-stat keys ride one packet, about 155 bytes.
pub(crate) fn encode(entity: i32, attributes: &[(u8, f64)], armor_stats: &[(u8, f64)]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(1 + 4 + 1 + attributes.len() * 9 + 1 + armor_stats.len() * 9);
    bytes.push(VERSION);
    bytes.extend_from_slice(&entity.to_le_bytes());
    bytes.push(attributes.len() as u8);
    for (index, value) in attributes { bytes.push(*index); bytes.extend_from_slice(&value.to_le_bytes()); }
    bytes.push(armor_stats.len() as u8);
    for (index, value) in armor_stats { bytes.push(*index); bytes.extend_from_slice(&value.to_le_bytes()); }
    bytes
}

fn read_entries(bytes: &[u8], offset: &mut usize, count: usize, bound: usize) -> Option<Vec<(u8, f64)>> {
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        if *offset + 9 > bytes.len() { return None; }
        let index = bytes[*offset];
        if index as usize >= bound { return None; }
        let value = f64::from_le_bytes(bytes[*offset + 1..*offset + 9].try_into().ok()?);
        if !value.is_finite() { return None; }
        entries.push((index, value));
        *offset += 9;
    }
    Some(entries)
}

/// The (entity, attribute values, armor-stat totals) a payload carries, or None when the
/// payload is not one of ours: the wrong version, too short, a bad index, a non-finite value,
/// or trailing bytes.
pub(crate) fn decode(bytes: &[u8]) -> Option<(i32, Vec<(u8, f64)>, Vec<(u8, f64)>)> {
    if bytes.len() < 7 || bytes[0] != VERSION { return None; }
    let entity = i32::from_le_bytes(bytes[1..5].try_into().ok()?);
    let mut offset = 5;
    let n_attributes = bytes[offset] as usize;
    offset += 1;
    let attributes = read_entries(bytes, &mut offset, n_attributes, custom_attributes::declared_count())?;
    if offset >= bytes.len() { return None; }
    let n_stats = bytes[offset] as usize;
    offset += 1;
    let armor_stats = read_entries(bytes, &mut offset, n_stats, custom_attributes::ARMOR_STAT_KEYS.len())?;
    if offset != bytes.len() { return None; }
    Some((entity, attributes, armor_stats))
}

/// The last values sent to each player, as (attributes, armor stats): the publish diffs
/// against these, so only changed values go out.
static LAST_SENT: Mutex<BTreeMap<u128, (BTreeMap<String, f64>, BTreeMap<String, f64>)>> = Mutex::new(BTreeMap::new());
/// The packets waiting for the end of the server tick, as (player, payload, failed ticks).
static OUTGOING: Mutex<Vec<(u128, Vec<u8>, u32)>> = Mutex::new(Vec::new());

/// Publishes a player's changed values for the end of the server tick: the attributes that
/// differ from their default against the last send, and the armor-stat totals that differ
/// from zero. A value that fell back to its default (or zero) goes out as the default (or
/// zero), so the guest forgets the old one. Nothing is queued when nothing changed. Called
/// from the custom-attributes tick, for every player, every tick.
pub(crate) fn publish(player: u128, entity: i32, stored: &BTreeMap<String, f64>, armor_stats: &[(String, f64)]) {
    let Ok(mut last_sent) = LAST_SENT.lock() else { return; };
    let last = last_sent.entry(player).or_default();
    let mut changed_attributes: Vec<(u8, f64)> = Vec::new();
    let mut next_attributes: BTreeMap<String, f64> = BTreeMap::new();
    for (attribute, value) in stored {
        next_attributes.insert(attribute.clone(), *value);
        if last.0.get(attribute) != Some(value) {
            if let Some(index) = custom_attributes::declared_index(attribute) { changed_attributes.push((index, *value)); }
        }
    }
    for attribute in last.0.keys() {
        if !stored.contains_key(attribute) {
            if let Some(index) = custom_attributes::declared_index(attribute) {
                if let Some(default) = custom_attributes::declared_default(attribute) { changed_attributes.push((index, default)); }
            }
        }
    }
    let current_stats: BTreeMap<String, f64> = armor_stats.iter().cloned().collect();
    let mut changed_stats: Vec<(u8, f64)> = Vec::new();
    for (key, amount) in &current_stats {
        if last.1.get(key) != Some(amount) {
            if let Some(index) = custom_attributes::ARMOR_STAT_KEYS.iter().position(|named| named == key).map(|index| index as u8) { changed_stats.push((index, *amount)); }
        }
    }
    for key in last.1.keys() {
        if !current_stats.contains_key(key) {
            if let Some(index) = custom_attributes::ARMOR_STAT_KEYS.iter().position(|named| named == key).map(|index| index as u8) { changed_stats.push((index, 0.0)); }
        }
    }
    if changed_attributes.is_empty() && changed_stats.is_empty() { return; }
    *last = (next_attributes, current_stats);
    let payload = encode(entity, &changed_attributes, &changed_stats);
    if let Ok(mut outgoing) = OUTGOING.lock() {
        // One packet per player per tick: a later publish in the same tick replaces the earlier one.
        if let Some(queued) = outgoing.iter_mut().find(|(queued_player, _, _)| *queued_player == player) {
            queued.1 = payload;
        } else {
            outgoing.push((player, payload, 0));
        }
    }
}

/// Sends the queued packets, at the end of the server tick. A failed send stays queued for
/// the next tick, and is dropped after MAX_FAILED_TICKS failures: a local player never
/// receives one, and the queue must not grow forever.
fn drain() {
    let Ok(mut outgoing) = OUTGOING.lock() else { return; };
    let mut kept = Vec::new();
    for (player, payload, failed) in outgoing.drain(..) {
        match ServerPlayNetworking::send(player, &channel(ATTRIBUTES_CHANNEL), &payload) {
            Ok(()) => {}
            Err(_) if failed + 1 < MAX_FAILED_TICKS => kept.push((player, payload, failed + 1)),
            Err(_) => {}
        }
    }
    *outgoing = kept;
}

/// The players whose guest clients asked for a double jump since the last tick, by uuid: the
/// server tick drains them into the pending double jump.
static DOUBLE_JUMP_REQUESTS: LazyLock<Mutex<HashSet<u128>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// Asks the server to run the extra jump for the local player: a guest client sends this
/// instead of resolving the jump locally, as it has no server tick. The payload is empty;
/// the sender is the request. Sending from a world with no server (or a vanilla one) fails
/// quietly and the jump simply does not happen.
pub(crate) fn request_double_jump() {
    let _ = ClientPlayNetworking::send(&channel(DOUBLE_JUMP_CHANNEL), &[]);
}

/// Whether the player asked for a double jump since the last tick, and clears the request:
/// the server tick calls this once per player per tick, and resolves a request through the
/// pending double jump the tick block already owns.
pub(crate) fn take_double_jump_request(player: u128) -> bool {
    DOUBLE_JUMP_REQUESTS.lock().map(|mut requests| requests.remove(&player)).unwrap_or(false)
}

/// Registers the receivers and the drain. Called once from the product's hooks, after the
/// custom attributes (the tick that publishes lives there).
pub(crate) fn register() {
    let _ = ClientPlayNetworking::register_global_receiver(channel(ATTRIBUTES_CHANNEL), |payload| {
        let Some((entity, attributes, armor_stats)) = decode(payload) else { return; };
        let attributes = attributes.into_iter().filter_map(|(index, value)| custom_attributes::declared_at(index).map(|id| (id.to_owned(), value))).collect::<BTreeMap<String, f64>>();
        let armor_stats = armor_stats.into_iter().filter_map(|(index, value)| custom_attributes::ARMOR_STAT_KEYS.get(index as usize).map(|key| (key.to_string(), value))).collect::<BTreeMap<String, f64>>();
        custom_attributes::apply_synced(entity, attributes, armor_stats);
    });
    let _ = ServerPlayNetworking::register_global_receiver(channel(DOUBLE_JUMP_CHANNEL), |player, _payload| {
        if let Ok(mut requests) = DOUBLE_JUMP_REQUESTS.lock() { requests.insert(player); }
    });
    ServerTickEvents::END_SERVER_TICK.register(|_| drain());
    ClientPlayConnectionEvents::DISCONNECT.register(|_| custom_attributes::clear_synced());
    ServerPlayConnectionEvents::DISCONNECT.register(|connection| {
        let player = connection.player.uuid;
        if let Ok(mut last_sent) = LAST_SENT.lock() { last_sent.remove(&player); }
        if let Ok(mut outgoing) = OUTGOING.lock() { outgoing.retain(|(queued, _, _)| *queued != player); }
        if let Ok(mut requests) = DOUBLE_JUMP_REQUESTS.lock() { requests.remove(&player); }
    });
    ServerLifecycleEvents::SERVER_STOPPED.register(|_| {
        if let Ok(mut last_sent) = LAST_SENT.lock() { last_sent.clear(); }
        if let Ok(mut outgoing) = OUTGOING.lock() { outgoing.clear(); }
        if let Ok(mut requests) = DOUBLE_JUMP_REQUESTS.lock() { requests.clear(); }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_payload_round_trips_and_rejects_bad_packets() {
        let bytes = encode(7708, &[(9, 2.0), (5, 0.5)], &[(1, 0.25)]);
        let (entity, attributes, armor_stats) = decode(&bytes).expect("a payload we made decodes");
        assert_eq!(entity, 7708);
        assert_eq!(attributes, vec![(9, 2.0), (5, 0.5)]);
        assert_eq!(armor_stats, vec![(1, 0.25)]);
        assert_eq!(decode(&encode(7708, &[], &[])).unwrap().1.len(), 0, "an empty change is a valid packet");
        assert!(decode(&[]).is_none());
        let mut wrong_version = bytes.clone();
        wrong_version[0] = 2;
        assert!(decode(&wrong_version).is_none(), "the wrong version is not ours");
        let mut bad_index = bytes.clone();
        bad_index[6] = 200;
        assert!(decode(&bad_index).is_none(), "an attribute index past the declarations is not ours");
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(decode(&trailing).is_none(), "trailing bytes are not ours");
        let mut truncated = bytes.clone();
        truncated.truncate(10);
        assert!(decode(&truncated).is_none());
        let nan = encode(7708, &[(9, f64::NAN)], &[]);
        assert!(decode(&nan).is_none(), "a non-finite value is not ours");
    }

    #[test]
    fn publish_queues_only_changed_values_and_the_drain_drops_a_failing_send() {
        let player = 12345u128;
        if let Ok(mut last_sent) = LAST_SENT.lock() { last_sent.remove(&player); }
        if let Ok(mut outgoing) = OUTGOING.lock() { outgoing.retain(|(queued, _, _)| *queued != player); }
        let mut stored = BTreeMap::new();
        stored.insert(custom_attributes::JUMP_COUNT.to_owned(), 2.0);
        publish(player, 7709, &stored, &[("tconstruct:jump_boost".to_owned(), 0.5)]);
        assert_eq!(OUTGOING.lock().unwrap().len(), 1, "the first change is queued");
        publish(player, 7709, &stored, &[("tconstruct:jump_boost".to_owned(), 0.5)]);
        assert_eq!(OUTGOING.lock().unwrap().len(), 1, "nothing changed, nothing more is queued");
        stored.insert(custom_attributes::JUMP_COUNT.to_owned(), 3.0);
        publish(player, 7709, &stored, &[("tconstruct:jump_boost".to_owned(), 0.5)]);
        assert_eq!(OUTGOING.lock().unwrap().len(), 1, "a later publish in the same tick replaces the queued packet");
        stored.remove(custom_attributes::JUMP_COUNT);
        publish(player, 7709, &stored, &[]);
        assert_eq!(OUTGOING.lock().unwrap().len(), 1, "the fall back to the default still goes out, so the guest forgets the old value");
        // The drain retries a failed send and drops it after the fifth failure: with no host
        // scope in a unit test, every send fails.
        for _ in 0..4 { drain(); }
        assert_eq!(OUTGOING.lock().unwrap().len(), 1, "still queued after four failures");
        drain();
        assert!(OUTGOING.lock().unwrap().is_empty(), "dropped after the fifth failure");
        if let Ok(mut last_sent) = LAST_SENT.lock() { last_sent.remove(&player); }
    }

    #[test]
    fn a_double_jump_request_is_taken_once() {
        let player = 67890u128;
        assert!(!take_double_jump_request(player), "no request yet");
        DOUBLE_JUMP_REQUESTS.lock().unwrap().insert(player);
        assert!(take_double_jump_request(player), "the request is there");
        assert!(!take_double_jump_request(player), "taking it clears it");
    }
}
