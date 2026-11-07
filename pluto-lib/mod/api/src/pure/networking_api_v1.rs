//! Counterpart of Fabric's `fabric-networking-api-v1` (connection events only, for now).

use super::base::Event;

/// What a connection callback receives. Fabric passes the network handler, a packet sender and the server; PlutoLib starts
/// with the player's name and grows as modules need more.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerConnection {
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

pub(crate) fn player_joined(name: String) {
    JOIN.invoke(&PlayerConnection { name });
}

pub(crate) fn player_left(name: String) {
    DISCONNECT.invoke(&PlayerConnection { name });
}
