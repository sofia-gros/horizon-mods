//! Counterpart of Fabric's `fabric-networking-api-v1`: connections and bounded play payloads.

use super::base::Event;
use super::player_api::{self, ServerPlayer};
use super::base::Identifier;
use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock, RwLock};

/// Native mod-transport cap in each direction. Larger logical messages need a protocol.
pub const MAX_PLAY_PAYLOAD: usize = 30_000;
type ServerReceiver = Arc<dyn Fn(u128, &[u8]) + Send + Sync>;
type ClientReceiver = Arc<dyn Fn(&[u8]) + Send + Sync>;
static SERVER_RECEIVERS: LazyLock<RwLock<BTreeMap<Identifier, ServerReceiver>>> = LazyLock::new(|| RwLock::new(BTreeMap::new()));
static CLIENT_RECEIVERS: LazyLock<RwLock<BTreeMap<Identifier, ClientReceiver>>> = LazyLock::new(|| RwLock::new(BTreeMap::new()));

fn check_channel(id: &Identifier) -> Result<(), String> {
    let namespace = id.namespace();
    let path = id.path();
    if namespace.is_empty() || namespace.len() > 64 || path.is_empty() || path.len() > 64
        || matches!(namespace, "minecraft" | "horizon")
        || !namespace.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        || !path.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'/' | b'-')) {
        return Err("play channel must be mod_id:name within native transport limits".into());
    }
    Ok(())
}
fn check_payload(id: &Identifier, data: &[u8]) -> Result<(), String> {
    check_channel(id)?;
    if data.len() > MAX_PLAY_PAYLOAD { return Err("play payload exceeds 30000 bytes".into()); }
    Ok(())
}

/// Server-side play payloads. Receivers run on the game callback thread, not the IO thread.
pub struct ServerPlayNetworking;
impl ServerPlayNetworking {
    /// Registers one receiver per channel. The first argument is the authenticated sender UUID.
    /// Register at startup; duplicate channels fail without replacing their owner.
    pub fn register_global_receiver(id: Identifier, receiver: impl Fn(u128, &[u8]) + Send + Sync + 'static) -> Result<(), String> {
        check_channel(&id)?;
        let mut receivers = SERVER_RECEIVERS.write().map_err(|_| "server payload registry poisoned")?;
        if receivers.contains_key(&id) { return Err(format!("payload receiver already registered: {id}")); }
        receivers.insert(id, Arc::new(receiver));
        Ok(())
    }
    /// Ok means queued, not delivered. Native transport permits 32 messages/player/tick;
    /// departed players and non-Horizon clients do not receive queued mod messages.
    /// Requires a scoped host callback; local single-player does not loop back.
    pub fn send(player: u128, channel: &Identifier, data: &[u8]) -> Result<(), String> {
        check_payload(channel, data)?;
        player_api::with_current(Err("no active network host scope".into()), |host| host.net_send_to_player(player, &channel.to_string(), data))
    }
}

/// Client-side play payloads; vanilla servers cannot receive this mod transport.
pub struct ClientPlayNetworking;
impl ClientPlayNetworking {
    pub fn register_global_receiver(id: Identifier, receiver: impl Fn(&[u8]) + Send + Sync + 'static) -> Result<(), String> {
        check_channel(&id)?;
        let mut receivers = CLIENT_RECEIVERS.write().map_err(|_| "client payload registry poisoned")?;
        if receivers.contains_key(&id) { return Err(format!("payload receiver already registered: {id}")); }
        receivers.insert(id, Arc::new(receiver));
        Ok(())
    }
    /// Requires a scoped host callback and a joined mod-capable server. Native rate limits apply.
    pub fn send(channel: &Identifier, data: &[u8]) -> Result<(), String> {
        check_payload(channel, data)?;
        player_api::with_current(Err("no active network host scope".into()), |host| host.net_send_to_server(&channel.to_string(), data))
    }
}

pub(crate) fn from_client(player: u128, channel: &str, data: &[u8]) {
    let Ok(id) = Identifier::parse(channel) else { return; };
    if check_payload(&id, data).is_err() { return; }
    let receiver = SERVER_RECEIVERS.read().ok().and_then(|registry| registry.get(&id).cloned());
    // Release the registry lock before user code (which may register another receiver).
    if let Some(receiver) = receiver { receiver(player, data); }
}
pub(crate) fn from_server(channel: &str, data: &[u8]) {
    let Ok(id) = Identifier::parse(channel) else { return; };
    if check_payload(&id, data).is_err() { return; }
    let receiver = CLIENT_RECEIVERS.read().ok().and_then(|registry| registry.get(&id).cloned());
    if let Some(receiver) = receiver { receiver(data); }
}

/// What a connection callback receives. Fabric passes the network handler, a packet sender and the server; PlutoLib passes the
/// player (whose methods work inside the callback) and grows as modules need more.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerConnection {
    pub player: ServerPlayer,
    /// The player's name (also `player.name`).
    pub name: String,
}

/// Fabric: `ServerPlayConnectionEvents`.
pub struct ServerPlayConnectionEvents;

static JOIN: Event<PlayerConnection, ()> = Event::void();
static DISCONNECT: Event<PlayerConnection, ()> = Event::void();

impl ServerPlayConnectionEvents {
    /// Fabric: `ServerPlayConnectionEvents.JOIN`: a player has joined the server.
    pub const JOIN: &'static Event<PlayerConnection, ()> = &JOIN;
    /// Fabric: `ServerPlayConnectionEvents.DISCONNECT`: a player has left the server.
    pub const DISCONNECT: &'static Event<PlayerConnection, ()> = &DISCONNECT;
}

pub(crate) fn player_joined(uuid: u128, name: String) {
    JOIN.invoke(&PlayerConnection { player: ServerPlayer { uuid, name: name.clone() }, name });
}

pub(crate) fn player_left(uuid: u128, name: String) {
    DISCONNECT.invoke(&PlayerConnection { player: ServerPlayer { uuid, name: name.clone() }, name });
}

/// Client connection boundaries, distinct from server player membership.
/// JOIN also fires when configuration sends a new login packet without disconnecting.
pub struct ClientPlayConnectionEvents;
static CLIENT_JOIN: Event<(), ()> = Event::void();
static CLIENT_DISCONNECT: Event<(), ()> = Event::void();
impl ClientPlayConnectionEvents {
    pub const JOIN: &'static Event<(), ()> = &CLIENT_JOIN;
    pub const DISCONNECT: &'static Event<(), ()> = &CLIENT_DISCONNECT;
}
pub(crate) fn client_joined() { CLIENT_JOIN.invoke(&()); }
pub(crate) fn client_disconnecting() { CLIENT_DISCONNECT.invoke(&()); }

#[cfg(test)]
mod payload_tests {
    use super::*;
    use std::sync::Mutex;
    #[test]
    fn routing_is_directional_bounded_and_keeps_the_first_owner() {
        let id = Identifier::new("pluto_test", "payload_routing").unwrap();
        let received = Arc::new(Mutex::new(Vec::new()));
        let output = received.clone();
        ServerPlayNetworking::register_global_receiver(id.clone(), move |player, data| output.lock().unwrap().push((player, data.to_vec()))).unwrap();
        assert!(ServerPlayNetworking::register_global_receiver(id.clone(), |_, _| panic!("replaced receiver")).is_err());
        from_server(&id.to_string(), b"wrong side");
        from_client(17, &id.to_string(), b"hello");
        from_client(18, &id.to_string(), &vec![0; MAX_PLAY_PAYLOAD + 1]);
        assert_eq!(*received.lock().unwrap(), vec![(17, b"hello".to_vec())]);
        let output = received.clone();
        ClientPlayNetworking::register_global_receiver(id.clone(), move |data| output.lock().unwrap().push((0, data.to_vec()))).unwrap();
        from_server(&id.to_string(), b"reply");
        assert_eq!(received.lock().unwrap().last().unwrap(), &(0, b"reply".to_vec()));
        assert!(ClientPlayNetworking::send(&id, b"outside host").is_err());
        assert!(ServerPlayNetworking::send(17, &id, b"outside host").is_err());
        assert!(check_payload(&id, &vec![0; MAX_PLAY_PAYLOAD]).is_ok());
        assert!(check_channel(&Identifier::new("minecraft", "reserved").unwrap()).is_err());
        assert!(check_channel(&Identifier::new("hyphen-mod", "valid_identifier_but_not_native_channel").unwrap()).is_err());
    }
}
