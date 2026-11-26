//! Generation-framed item registry/tag transfer over PlutoLib's bounded play transport: the
//! server's captured item snapshot republished to remote clients. A remote client's own reload
//! callbacks never see a registry snapshot (the native reload driver hands non-authoritative
//! games an empty one), so without this transfer every item-tag query on the guest answers
//! false: a broad tool fails the `tconstruct:modifiable/broad` predicate and passes its
//! inversion instead, losing the Overwield mining speed bonus (issue #363). The host and
//! single-player read the local capture as before; only a remote client falls back to the
//! snapshot its server published here, exactly as material state does in `material_sync`.
use pluto_lib::{ClientPlayConnectionEvents, ClientPlayNetworking, Identifier, RegistryLookup, ServerLifecycleEvents, ServerPlayConnectionEvents, ServerPlayNetworking, ServerTickEvents};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

const HEADER: usize = 16;
const CHUNK: usize = pluto_lib::MAX_PLAY_PAYLOAD - HEADER;
const PER_PLAYER_TICK: usize = 8;
/// The encoded snapshot's size cap. The whole vanilla and Tinkers item registry with its tags
/// sits far below this; anything larger is refused rather than streamed.
pub(crate) const MAX_BYTES: usize = 4 * 1024 * 1024;
/// The snapshot the server currently publishes: generation, the lookup it was encoded from
/// (kept so an unchanged recapture is recognized) and the encoded bytes.
static PUBLISHED: RwLock<Option<(u64, Arc<RegistryLookup>, Arc<[u8]>)>> = RwLock::new(None);
/// The snapshot this client received from its server; None while no transfer completed.
static CLIENT: OnceLock<Arc<RwLock<Option<Arc<RegistryLookup>>>>> = OnceLock::new();

fn channel() -> Identifier { Identifier::new("tconstruct", "item_tags/v1").expect("constant channel") }
fn frame(generation: u64, bytes: &[u8], offset: usize) -> Vec<u8> {
    let end = (offset + CHUNK).min(bytes.len());
    let mut packet = Vec::with_capacity(HEADER + end - offset);
    packet.extend_from_slice(&generation.to_be_bytes());
    packet.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    packet.extend_from_slice(&(offset as u32).to_be_bytes());
    packet.extend_from_slice(&bytes[offset..end]);
    packet
}

/// The snapshot as one bounded JSON document: every entry, then every tag with its members,
/// the same parts `RegistryLookup::from_parts` rebuilds on the other side.
pub(crate) fn encode(lookup: &RegistryLookup) -> Result<Vec<u8>, String> {
    let entries: Vec<String> = lookup.entries().iter().map(ToString::to_string).collect();
    let tags: Vec<(String, Vec<String>)> = lookup.tags().iter()
        .map(|(tag, members)| (tag.to_string(), members.iter().map(ToString::to_string).collect())).collect();
    let bytes = serde_json::to_vec(&serde_json::json!({ "entries": entries, "tags": tags })).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_BYTES { return Err("item tag snapshot exceeds 4 MiB".into()); }
    Ok(bytes)
}

pub(crate) fn decode(bytes: &[u8]) -> Result<RegistryLookup, String> {
    if bytes.len() > MAX_BYTES { return Err("item tag snapshot exceeds 4 MiB".into()); }
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| format!("invalid item tag snapshot: {error}"))?;
    let object = value.as_object().ok_or("item tag snapshot must be an object")?;
    let string = |value: &serde_json::Value| value.as_str().map(str::to_owned).ok_or_else(|| "item tag snapshot holds a non-string ID".to_string());
    let entries = object.get("entries").and_then(serde_json::Value::as_array).ok_or("item tag snapshot entries missing")?;
    if entries.len() > 200_000 { return Err("item tag snapshot has too many entries".into()); }
    let entries = entries.iter().map(string).collect::<Result<Vec<String>, _>>()?;
    let tags = object.get("tags").and_then(serde_json::Value::as_array).ok_or("item tag snapshot tags missing")?;
    if tags.len() > 20_000 { return Err("item tag snapshot has too many tags".into()); }
    let mut parsed = Vec::with_capacity(tags.len());
    for pair in tags {
        let pair = pair.as_array().filter(|pair| pair.len() == 2).ok_or("item tag snapshot tag must be an ID and its members")?;
        let members = pair[1].as_array().ok_or("item tag snapshot tag members must be an array")?;
        if members.len() > 200_000 { return Err("item tag snapshot tag has too many members".into()); }
        parsed.push((string(&pair[0])?, members.iter().map(string).collect::<Result<Vec<String>, _>>()?));
    }
    RegistryLookup::from_parts(&entries, &parsed)
}

/// Republishes the server's captured item snapshot for joined and joining remote clients.
/// Called on every reload capture; an unchanged snapshot keeps its generation, so the many
/// listener captures of one reload start at most one transfer.
pub(crate) fn publish(lookup: &RegistryLookup) {
    let mut published = PUBLISHED.write().expect("item tag publish lock poisoned");
    if let Some((_, current, _)) = &*published { if current.as_ref() == lookup { return; } }
    let bytes = match encode(lookup) { Ok(bytes) => bytes, Err(error) => { eprintln!("Tinkers item tag sync: {error}"); return; } };
    let generation = published.as_ref().map_or(1, |(generation, _, _)| generation + 1);
    *published = Some((generation, Arc::new(lookup.clone()), Arc::from(bytes)));
}

/// Drops the published snapshot when the server stops, so a later server in this process
/// never streams the previous world's tags.
pub(crate) fn clear_published() { *PUBLISHED.write().expect("item tag publish lock poisoned") = None; }

/// The item snapshot this client received from its server, if a transfer completed.
pub(crate) fn client_snapshot() -> Option<Arc<RegistryLookup>> {
    CLIENT.get()?.read().expect("client item tag lock poisoned").clone()
}

#[derive(Default)]
struct Receiver { pending: Option<(u64, usize, Vec<u8>)> }
impl Receiver {
    fn push(&mut self, packet: &[u8]) -> Result<(bool, Option<RegistryLookup>), String> {
        let result = self.accept(packet);
        if result.is_err() { self.pending = None; }
        result
    }
    fn accept(&mut self, packet: &[u8]) -> Result<(bool, Option<RegistryLookup>), String> {
        if packet.len() <= HEADER || packet.len() > pluto_lib::MAX_PLAY_PAYLOAD { return Err("invalid item tag frame size".into()); }
        let generation = u64::from_be_bytes(packet[..8].try_into().unwrap());
        let total = u32::from_be_bytes(packet[8..12].try_into().unwrap()) as usize;
        let offset = u32::from_be_bytes(packet[12..16].try_into().unwrap()) as usize;
        let data = &packet[HEADER..];
        if total == 0 || total > MAX_BYTES || offset > total || data.len() > total - offset {
            return Err("invalid item tag frame bounds".into());
        }
        let started = offset == 0;
        if started { self.pending = Some((generation, total, Vec::new())); }
        let (pending_generation, pending_total, bytes) = self.pending.as_mut().ok_or("item tag frame has no beginning")?;
        if *pending_generation != generation || *pending_total != total || bytes.len() != offset {
            return Err("item tag frame generation or order mismatch".into());
        }
        bytes.extend_from_slice(data);
        if bytes.len() != total { return Ok((started, None)); }
        let (_, _, bytes) = self.pending.take().unwrap();
        Ok((started, Some(decode(&bytes)?)))
    }
}

#[derive(Default)]
struct Outgoing { generation: Option<u64>, bytes: Option<Arc<[u8]>>, offset: usize }
impl Outgoing {
    fn update(&mut self, generation: u64, bytes: &Option<Arc<[u8]>>) {
        if self.generation != Some(generation) {
            self.generation = Some(generation);
            self.bytes = bytes.clone();
            self.offset = 0;
        }
    }
    fn send(&mut self, mut send: impl FnMut(&[u8]) -> Result<(), String>) {
        let Some(bytes) = self.bytes.as_ref() else { return; };
        for _ in 0..PER_PLAYER_TICK {
            let packet = frame(self.generation.expect("prepared generation"), bytes, self.offset);
            // Keep the unsent offset when the shared native tick allowance is exhausted.
            if send(&packet).is_err() { return; }
            self.offset = (self.offset + CHUNK).min(bytes.len());
            if self.offset == bytes.len() { self.bytes = None; return; }
        }
    }
}

pub(crate) fn register() {
    let client = CLIENT.get_or_init(|| Arc::new(RwLock::new(None))).clone();
    let receiver = Arc::new(Mutex::new(Receiver::default()));
    let incoming = receiver.clone();
    let output = client.clone();
    ClientPlayNetworking::register_global_receiver(channel(), move |packet| {
        let result = incoming.lock().expect("item tag receive lock poisoned").push(packet);
        let mut state = output.write().expect("client item tag lock poisoned");
        match result {
            Ok((started, completed)) => {
                if started { *state = None; }
                if let Some(completed) = completed { *state = Some(Arc::new(completed)); }
            }
            Err(error) => { *state = None; eprintln!("Tinkers item tag sync: {error}"); }
        }
    }).expect("item tag receiver registration failed");
    // Pinned Horizon invokes client JOIN only from remote Login handling, not local worlds.
    for event in [ClientPlayConnectionEvents::JOIN, ClientPlayConnectionEvents::DISCONNECT] {
        let receiver = receiver.clone();
        let client = client.clone();
        event.register(move |_| {
            *receiver.lock().expect("item tag receive lock poisoned") = Receiver::default();
            *client.write().expect("client item tag lock poisoned") = None;
        });
    }
    let players = Arc::new(Mutex::new(std::collections::BTreeMap::<u128, Outgoing>::new()));
    let joined = players.clone();
    ServerPlayConnectionEvents::JOIN.register(move |connection| {
        joined.lock().expect("item tag send lock poisoned").insert(connection.player.uuid, Outgoing::default());
    });
    let left = players.clone();
    ServerPlayConnectionEvents::DISCONNECT.register(move |connection| {
        left.lock().expect("item tag send lock poisoned").remove(&connection.player.uuid);
    });
    let stopped = players.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| stopped.lock().expect("item tag send lock poisoned").clear());
    ServerTickEvents::END_SERVER_TICK.register(move |_| {
        let current = {
            let published = PUBLISHED.read().expect("item tag publish lock poisoned");
            published.as_ref().map(|(generation, _, bytes)| (*generation, bytes.clone()))
        };
        let mut players = players.lock().expect("item tag send lock poisoned");
        for (player, pending) in players.iter_mut() {
            if let Some((generation, bytes)) = &current { pending.update(*generation, &Some(bytes.clone())); }
            pending.send(|packet| ServerPlayNetworking::send(*player, &channel(), packet));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> RegistryLookup {
        RegistryLookup::from_parts(
            &["tconstruct:sledge_hammer".to_owned(), "tconstruct:pickaxe".to_owned(), "minecraft:stick".to_owned()],
            &[("tconstruct:modifiable/broad".to_owned(), vec!["tconstruct:sledge_hammer".to_owned()]),
              ("tconstruct:modifiable/harvest".to_owned(), vec!["tconstruct:sledge_hammer".to_owned(), "tconstruct:pickaxe".to_owned()]),
              ("test:empty".to_owned(), vec![])],
        ).unwrap()
    }

    #[test]
    fn encode_decode_roundtrips_entries_and_tags() {
        let lookup = snapshot();
        let decoded = decode(&encode(&lookup).unwrap()).unwrap();
        assert_eq!(decoded, lookup);
        let broad = Identifier::parse("tconstruct:modifiable/broad").unwrap();
        let hammer = Identifier::parse("tconstruct:sledge_hammer").unwrap();
        let pickaxe = Identifier::parse("tconstruct:pickaxe").unwrap();
        // The classification issue #363 turns on: the broad tool is in the tag, the narrow one is not,
        // and an absent tag stays absent rather than becoming an empty answer.
        assert!(decoded.get_tag(&broad).unwrap().contains(&hammer));
        assert!(!decoded.get_tag(&broad).unwrap().contains(&pickaxe));
        assert!(decoded.get_tag(&Identifier::parse("test:empty").unwrap()).unwrap().is_empty());
        assert!(decoded.get_tag(&Identifier::parse("test:absent").unwrap()).is_none());
        assert!(decode(b"not json").is_err());
        assert!(decode(&vec![b'x'; MAX_BYTES + 1]).is_err());
        assert!(decode(br#"{"entries":["a:b"],"tags":[["t:c",["a:missing"]]]}"#).is_err(), "a member that is no entry rejects the snapshot");
    }

    #[test]
    fn reassembly_is_atomic_bounded_and_rejects_mixed_generations() {
        let bytes = encode(&snapshot()).unwrap();
        let mut receiver = Receiver::default();
        let mut first = frame(3, &bytes, 0);
        let cut = HEADER + bytes.len() / 2;
        first.truncate(cut);
        assert!(receiver.push(&first).unwrap().1.is_none());
        assert!(receiver.push(&frame(4, &bytes, cut - HEADER)).is_err());
        assert!(receiver.pending.is_none());
        assert!(receiver.push(&frame(3, &bytes, cut - HEADER)).is_err());
        assert_eq!(receiver.push(&frame(5, &bytes, 0)).unwrap().1.unwrap(), snapshot());
        let mut oversized = frame(6, &bytes, 0);
        oversized[8..12].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(receiver.push(&oversized).is_err());
    }

    #[test]
    fn publish_keeps_the_generation_of_an_unchanged_snapshot() {
        clear_published();
        publish(&snapshot());
        let first = PUBLISHED.read().unwrap().as_ref().map(|(generation, _, _)| *generation);
        publish(&snapshot());
        let second = PUBLISHED.read().unwrap().as_ref().map(|(generation, _, _)| *generation);
        assert_eq!(first, second, "recapturing the same snapshot starts no new transfer");
        let changed = RegistryLookup::from_parts(&["minecraft:stick".to_owned()], &[]).unwrap();
        publish(&changed);
        let third = PUBLISHED.read().unwrap().as_ref().map(|(generation, _, _)| *generation);
        assert!(third > second, "a changed snapshot is a new generation");
        clear_published();
        assert!(PUBLISHED.read().unwrap().is_none());
    }

    #[test]
    fn outgoing_retries_without_skipping_and_reloads_replace_pending_bytes() {
        let data: Arc<[u8]> = vec![42; CHUNK * 10].into();
        let mut outgoing = Outgoing::default();
        outgoing.update(1, &Some(data.clone()));
        outgoing.send(|_| Err("native tick quota".into()));
        assert_eq!(outgoing.offset, 0);
        let mut packets = Vec::new();
        outgoing.send(|packet| { packets.push(packet.to_vec()); Ok(()) });
        assert_eq!(packets.len(), PER_PLAYER_TICK);
        assert_eq!(outgoing.offset, CHUNK * PER_PLAYER_TICK);
        outgoing.update(2, &Some(Arc::from(&b"replacement"[..])));
        assert_eq!(outgoing.offset, 0);
        outgoing.send(|packet| { assert_eq!(&packet[HEADER..], b"replacement"); Ok(()) });
        assert!(outgoing.bytes.is_none());
        outgoing.update(2, &Some(data));
        assert!(outgoing.bytes.is_none(), "completed generation is not resent every tick");
    }
}
