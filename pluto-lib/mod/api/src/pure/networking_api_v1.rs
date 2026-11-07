//! Counterpart of Fabric's `fabric-networking-api-v1` (connection events only, for now).

use super::base::Event;
use super::player_api::ServerPlayer;

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
