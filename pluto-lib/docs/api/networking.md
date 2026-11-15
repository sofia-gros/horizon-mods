# Networking: player connections and play payloads

Be told when a player joins or leaves. Callbacks get a `PlayerConnection` whose `player` can be used right away to message or give items (see `player.md`).

## Basic Example

Greet a player when they join:

```rust
use pluto_lib::ServerPlayConnectionEvents;

ServerPlayConnectionEvents::JOIN.register(|join| {
    join.player.send_message("Welcome to the server!");
});
```

## Giving Something Once

`JOIN` fires on every join, so a one-time gift needs a flag that is saved with the player (see [attachments](attachments.md)):

```rust
use pluto_lib::{AttachmentRegistry, Identifier, ServerPlayConnectionEvents};

ServerPlayConnectionEvents::JOIN.register(|join| {
    let gift = AttachmentRegistry::create(Identifier::new("example", "gift_given").unwrap(), || false);
    if !join.player.get_attached(&gift) && join.player.give_item("minecraft:bread", 3) == 0 {
        join.player.set_attached(&gift, true);
    }
});
```

> **Use with Caution.** The `ServerPlayer` is valid inside the callback. A copy kept for later reports failure instead of acting (see [player](player.md)).

## Bounded play payloads

Register receivers during mod setup; send inside a host-scoped callback such as `JOIN` or a payload receiver. Receivers execute on the game callback thread, with no registry lock held. Server receivers receive the authenticated sender UUID, not a UUID supplied by the payload.

```rust
use pluto_lib::{ClientPlayNetworking, Identifier, ServerPlayNetworking};
let channel = Identifier::new("example", "state").unwrap();
ServerPlayNetworking::register_global_receiver(channel.clone(), |player, bytes| {
    let channel = Identifier::new("example", "state").unwrap();
    // Validate your own message format before acting on it.
    let _queued = ServerPlayNetworking::send(player, &channel, bytes);
}).unwrap();
ClientPlayNetworking::register_global_receiver(channel, |bytes| {
    let _received = bytes;
}).unwrap();
```

`ClientPlayNetworking::send(&channel, bytes)` sends to a joined mod-capable server. Both directions limit each message to `MAX_PLAY_PAYLOAD` (30,000 bytes). Channels require a namespace of 1–64 lowercase ASCII letters, digits or underscores (not `minecraft` or `horizon`) and a path of 1–64 lowercase ASCII letters, digits, `_`, `.`, `/` or `-`. These are narrower than general `Identifier` rules. Duplicate receiver registration fails without replacing the first receiver.

The host enforces at most 32 mod messages per player per tick. `Ok(())` means **queued**, not delivered: disconnected players and non-Horizon clients do not receive server messages. The local integrated player does not loop back through this transport. Calls outside a host scope, premature client sends, unsupported servers, invalid channels and oversized messages return errors. No fragmentation, acknowledgments, retry queue, version negotiation or material synchronization is implied. Treat incoming bytes as untrusted and bound decoded allocations. Host-independent tests cover routing/caps; LAN delivery remains subject to native/game validation.

## Reference

### `ClientPlayConnectionEvents`

Client login/disconnection boundaries, distinct from the server's player membership events. No fabricated player handle is supplied. Use these to discard connection-owned caches or incomplete transfers.

```rust
use pluto_lib::ClientPlayConnectionEvents;
ClientPlayConnectionEvents::DISCONNECT.register(|_| {});
```

### `ClientPlayConnectionEvents::JOIN`

Fires after the client's login packet, including a new login after reconfiguration without a prior disconnect. The player may not yet be available. It is not a guarantee that a server accepts mod messages.

```rust
use pluto_lib::ClientPlayConnectionEvents;
ClientPlayConnectionEvents::JOIN.register(|_| {});
```

### `ClientPlayConnectionEvents::DISCONNECT`

Fires before an actual connection is discarded, not for a dimension change. This callback has no mutable host scope; clear local state, do not send.

```rust
use pluto_lib::ClientPlayConnectionEvents;
ClientPlayConnectionEvents::DISCONNECT.register(|_| {});
```

### `MAX_PLAY_PAYLOAD`

Maximum data bytes in either direction, before the native transport envelope.

```rust
use pluto_lib::MAX_PLAY_PAYLOAD;
let data = vec![0u8; MAX_PLAY_PAYLOAD];
```

### `ServerPlayNetworking`

Server receiver registration and targeted sends; see the transport limitations above.

```rust
use pluto_lib::ServerPlayNetworking;
let _api = ServerPlayNetworking;
```

### `ServerPlayNetworking::register_global_receiver`

Takes a channel `Identifier` and a thread-safe static callback receiving sender UUID and borrowed bytes. Duplicate registration returns an error.

```rust
use pluto_lib::{Identifier, ServerPlayNetworking};
ServerPlayNetworking::register_global_receiver(Identifier::new("example", "request").unwrap(), |uuid, data| {
    let _request = (uuid, data);
}).unwrap();
```

### `ServerPlayNetworking::send`

Queues bytes for the player's UUID; returns native queue errors unchanged. This example is intentionally outside a live callback and therefore fails.

```rust
use pluto_lib::{Identifier, ServerPlayNetworking};
assert!(ServerPlayNetworking::send(7, &Identifier::new("example", "state").unwrap(), b"state").is_err());
```

### `ClientPlayNetworking`

Client receiver registration and sends to the connected server.

```rust
use pluto_lib::ClientPlayNetworking;
let _api = ClientPlayNetworking;
```

### `ClientPlayNetworking::register_global_receiver`

Takes a channel and a thread-safe static callback receiving borrowed server bytes.

```rust
use pluto_lib::{Identifier, ClientPlayNetworking};
ClientPlayNetworking::register_global_receiver(Identifier::new("example", "state").unwrap(), |data| {
    let _received = data;
}).unwrap();
```

### `ClientPlayNetworking::send`

Queues bytes to a joined mod-capable server from a host-scoped callback. No connection or scope returns an error.

```rust
use pluto_lib::{Identifier, ClientPlayNetworking};
assert!(ClientPlayNetworking::send(&Identifier::new("example", "request").unwrap(), b"request").is_err());
```

### `PlayerConnection`

What a connection callback receives. `player` is the `ServerPlayer` (its methods work inside the callback), `name` is the player's name (also `player.name`).

```rust
use pluto_lib::{PlayerConnection, ServerPlayConnectionEvents, ServerPlayer};

ServerPlayConnectionEvents::JOIN.register(|connection: &PlayerConnection| {
    let _uuid = connection.player.uuid;
    println!("{} joined", connection.name);
});
ServerPlayConnectionEvents::JOIN.invoke(&PlayerConnection {
    player: ServerPlayer { uuid: 1, name: "Alex".into() },
    name: "Alex".into(),
});
```

### `ServerPlayConnectionEvents`

The two connection events.

```rust
use pluto_lib::ServerPlayConnectionEvents;

ServerPlayConnectionEvents::JOIN.register(|_| {});
ServerPlayConnectionEvents::DISCONNECT.register(|_| {});
```

### `ServerPlayConnectionEvents::JOIN`

A player joined the server. The usual place to greet a player or give a starting item; the player handle works inside this callback.

```rust
use pluto_lib::{PlayerConnection, ServerPlayConnectionEvents, ServerPlayer};

ServerPlayConnectionEvents::JOIN.register(|join| {
    // outside a running game these report failure instead of acting
    let told = join.player.send_message("Welcome!");
    let left = join.player.give_item("minecraft:bread", 3);
    assert!(!told);
    assert_eq!(left, 3);
});
ServerPlayConnectionEvents::JOIN.invoke(&PlayerConnection {
    player: ServerPlayer { uuid: 7, name: "Sam".into() },
    name: "Sam".into(),
});
```

### `ServerPlayConnectionEvents::DISCONNECT`

A player left the server. The handle is still valid inside the callback.

```rust
use pluto_lib::{PlayerConnection, ServerPlayConnectionEvents, ServerPlayer};

ServerPlayConnectionEvents::DISCONNECT.register(|leave| println!("{} left", leave.name));
ServerPlayConnectionEvents::DISCONNECT.invoke(&PlayerConnection {
    player: ServerPlayer { uuid: 7, name: "Sam".into() },
    name: "Sam".into(),
});
```
