//! Generation-framed material transfer over PlutoLib's bounded play transport.
use crate::{material_snapshot, materials::Definitions};
use pluto_lib::{ClientPlayConnectionEvents, ClientPlayNetworking, Identifier, ServerLifecycleEvents, ServerPlayConnectionEvents, ServerPlayNetworking, ServerTickEvents};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

const HEADER: usize = 16;
const CHUNK: usize = pluto_lib::MAX_PLAY_PAYLOAD - HEADER;
const PER_PLAYER_TICK: usize = 8;
static CLIENT: OnceLock<Arc<RwLock<Option<Definitions>>>> = OnceLock::new();
static REMOTE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub(crate) fn remote_connection() -> bool { REMOTE.load(std::sync::atomic::Ordering::Acquire) }
pub(crate) fn client_state() -> Option<&'static Arc<RwLock<Option<Definitions>>>> { CLIENT.get() }
fn channel() -> Identifier { Identifier::new("tconstruct", "materials/v1").expect("constant channel") }
fn frame(generation: u64, bytes: &[u8], offset: usize) -> Vec<u8> {
    let end = (offset + CHUNK).min(bytes.len());
    let mut packet = Vec::with_capacity(HEADER + end - offset);
    packet.extend_from_slice(&generation.to_be_bytes());
    packet.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    packet.extend_from_slice(&(offset as u32).to_be_bytes());
    packet.extend_from_slice(&bytes[offset..end]);
    packet
}

#[derive(Default)]
struct Receiver { pending: Option<(u64, usize, Vec<u8>)> }
impl Receiver {
    fn push(&mut self, packet: &[u8]) -> Result<(bool, Option<Definitions>), String> {
        let result = self.accept(packet);
        if result.is_err() { self.pending = None; }
        result
    }
    fn accept(&mut self, packet: &[u8]) -> Result<(bool, Option<Definitions>), String> {
        if packet.len() <= HEADER || packet.len() > pluto_lib::MAX_PLAY_PAYLOAD { return Err("invalid material frame size".into()); }
        let generation = u64::from_be_bytes(packet[..8].try_into().unwrap());
        let total = u32::from_be_bytes(packet[8..12].try_into().unwrap()) as usize;
        let offset = u32::from_be_bytes(packet[12..16].try_into().unwrap()) as usize;
        let data = &packet[HEADER..];
        if total == 0 || total > material_snapshot::MAX_BYTES || offset > total || data.len() > total - offset {
            return Err("invalid material frame bounds".into());
        }
        let started = offset == 0;
        if started { self.pending = Some((generation, total, Vec::new())); }
        let (pending_generation, pending_total, bytes) = self.pending.as_mut().ok_or("material frame has no beginning")?;
        if *pending_generation != generation || *pending_total != total || bytes.len() != offset {
            return Err("material frame generation or order mismatch".into());
        }
        bytes.extend_from_slice(data);
        if bytes.len() != total { return Ok((started, None)); }
        let (_, _, bytes) = self.pending.take().unwrap();
        let mut decoded = material_snapshot::decode(&bytes)?;
        decoded.generation = generation;
        Ok((started, Some(decoded)))
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

pub(crate) fn register(server: Arc<RwLock<Definitions>>) {
    let client = CLIENT.get_or_init(|| Arc::new(RwLock::new(None))).clone();
    let receiver = Arc::new(Mutex::new(Receiver::default()));
    let incoming = receiver.clone();
    let output = client.clone();
    ClientPlayNetworking::register_global_receiver(channel(), move |packet| {
        let result = incoming.lock().expect("material receive lock poisoned").push(packet);
        let mut state = output.write().expect("client material lock poisoned");
        let mut notification = None;
        match result {
            Ok((started, completed)) => {
                if started { *state = None; }
                if let Some(completed) = completed {
                    if completed.server_loaded {
                        notification = Some(crate::materials::MaterialsLoaded {
                            side: crate::materials::MaterialSide::RemoteClient, generation: completed.generation,
                        });
                    }
                    *state = Some(completed);
                }
            }
            Err(error) => { *state = None; eprintln!("Tinkers material sync: {error}"); }
        }
        drop(state);
        if let Some(notification) = notification { crate::materials::MATERIALS_LOADED.invoke(&notification); }
    }).expect("material receiver registration failed");
    // Pinned Horizon invokes client JOIN only from remote Login handling, not local worlds.
    for (event, remote) in [(ClientPlayConnectionEvents::JOIN, true), (ClientPlayConnectionEvents::DISCONNECT, false)] {
        let receiver = receiver.clone();
        let client = client.clone();
        event.register(move |_| {
            REMOTE.store(remote, std::sync::atomic::Ordering::Release);
            *receiver.lock().expect("material receive lock poisoned") = Receiver::default();
            *client.write().expect("client material lock poisoned") = None;
        });
    }
    let players = Arc::new(Mutex::new(BTreeMap::<u128, Outgoing>::new()));
    let joined = players.clone();
    ServerPlayConnectionEvents::JOIN.register(move |connection| {
        joined.lock().expect("material send lock poisoned").insert(connection.player.uuid, Outgoing::default());
    });
    let left = players.clone();
    ServerPlayConnectionEvents::DISCONNECT.register(move |connection| {
        left.lock().expect("material send lock poisoned").remove(&connection.player.uuid);
    });
    let stopped = players.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| stopped.lock().expect("material send lock poisoned").clear());
    ServerTickEvents::END_SERVER_TICK.register(move |_| {
        let (generation, bytes) = {
            let state = server.read().expect("server material lock poisoned");
            (state.generation, state.snapshot.clone())
        };
        let mut players = players.lock().expect("material send lock poisoned");
        for (player, pending) in players.iter_mut() {
            pending.update(generation, &bytes);
            pending.send(|packet| ServerPlayNetworking::send(*player, &channel(), packet));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reassembly_is_atomic_bounded_and_rejects_mixed_generations() {
        let state = Definitions::default();
        let bytes = material_snapshot::encode(&state, &[], &[]).unwrap();
        let mut receiver = Receiver::default();
        let mut first = frame(3, &bytes, 0);
        let cut = HEADER + bytes.len()/2;
        first.truncate(cut);
        assert!(receiver.push(&first).unwrap().1.is_none());
        assert!(receiver.push(&frame(4, &bytes, cut-HEADER)).is_err());
        assert!(receiver.pending.is_none());
        assert!(receiver.push(&frame(3, &bytes, cut-HEADER)).is_err());
        assert_eq!(receiver.push(&frame(5, &bytes, 0)).unwrap().1.unwrap().generation, 5);
        let mut oversized = frame(6, &bytes, 0);
        oversized[8..12].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(receiver.push(&oversized).is_err());
    }
    #[test]
    fn outgoing_retries_without_skipping_and_reloads_replace_pending_bytes() {
        let data: Arc<[u8]> = vec![42; CHUNK*10].into();
        let mut outgoing = Outgoing::default();
        outgoing.update(1, &Some(data.clone()));
        outgoing.send(|_| Err("native tick quota".into()));
        assert_eq!(outgoing.offset, 0);
        let mut packets = Vec::new();
        outgoing.send(|packet| { packets.push(packet.to_vec()); Ok(()) });
        assert_eq!(packets.len(), PER_PLAYER_TICK);
        assert_eq!(outgoing.offset, CHUNK*PER_PLAYER_TICK);
        outgoing.update(2, &Some(Arc::from(&b"replacement"[..])));
        assert_eq!(outgoing.offset, 0);
        outgoing.send(|packet| { assert_eq!(&packet[HEADER..], b"replacement"); Ok(()) });
        assert!(outgoing.bytes.is_none());
        outgoing.update(2, &Some(data));
        assert!(outgoing.bytes.is_none(), "completed generation is not resent every tick");
    }
}
