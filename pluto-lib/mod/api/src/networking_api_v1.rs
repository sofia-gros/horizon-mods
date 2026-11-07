//! Counterpart of Fabric's `fabric-networking-api-v1` (connection events only, for now).

use crate::base::Event;

/// What a connection callback receives. Fabric passes the network handler, a packet sender and the server; PlutoLib starts
/// with the player's name and grows as modules need more.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerConnection {
    pub name: String,
}

/// Fabric: `ServerPlayConnectionEvents`.
pub struct ServerPlayConnectionEvents;

static JOIN: Event<PlayerConnection, ()> = Event::void();

impl ServerPlayConnectionEvents {
    /// Fabric: `ServerPlayConnectionEvents.JOIN`: a player has joined the server.
    pub const JOIN: &'static Event<PlayerConnection, ()> = &JOIN;
}

pub(crate) fn player_joined(name: String) {
    JOIN.invoke(&PlayerConnection { name });
}
