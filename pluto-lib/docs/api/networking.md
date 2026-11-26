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

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: absent](badges/sunrise-absent.svg)

Client login/disconnection boundaries, distinct from the server's player membership events.

> No fabricated player handle is supplied. Use these to discard connection-owned caches or
> incomplete transfers.

```rust
use pluto_lib::ClientPlayConnectionEvents;
ClientPlayConnectionEvents::DISCONNECT.register(|_| {});
```

### `ClientPlayConnectionEvents::JOIN`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: absent](badges/sunrise-absent.svg)

Fires after the client's login packet, including a new login after reconfiguration without a prior disconnect.

> The player may not yet be available. It is not a guarantee that a server accepts mod
> messages.

```rust
use pluto_lib::ClientPlayConnectionEvents;
ClientPlayConnectionEvents::JOIN.register(|_| {});
```

### `ClientPlayConnectionEvents::DISCONNECT`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: absent](badges/sunrise-absent.svg)

Fires before an actual connection is discarded, not for a dimension change.

> This callback has no mutable host scope; clear local state, do not send.

```rust
use pluto_lib::ClientPlayConnectionEvents;
ClientPlayConnectionEvents::DISCONNECT.register(|_| {});
```

### `MAX_PLAY_PAYLOAD`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Maximum data bytes in either direction, before the native transport envelope.

> The cap counts your data bytes only; the transport's own envelope rides on top of it. A
> transfer bigger than the cap has to be split into several messages, each under it, in
> either direction.

```rust
use pluto_lib::MAX_PLAY_PAYLOAD;
let data = vec![0u8; MAX_PLAY_PAYLOAD];
```

### `ServerPlayNetworking`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Server receiver registration and targeted sends.

> Register receivers with `register_global_receiver` — a channel and a callback that hears
> the sender's UUID and the message bytes — and reach one player with `send`, which queues
> bytes for that player's UUID. Registering the same channel twice returns an error.

```rust
use pluto_lib::ServerPlayNetworking;
let _api = ServerPlayNetworking;
```

### `ServerPlayNetworking::register_global_receiver`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Takes a channel `Identifier` and a thread-safe static callback receiving sender UUID and borrowed bytes.

> Registering the same channel twice returns an error rather than replacing the first
> receiver. The callback hears the sender's UUID with the borrowed bytes, so it can answer
> a specific player through `send`.

```rust
use pluto_lib::{Identifier, ServerPlayNetworking};
ServerPlayNetworking::register_global_receiver(Identifier::new("example", "request").unwrap(), |uuid, data| {
    let _request = (uuid, data);
}).unwrap();
```

### `ServerPlayNetworking::send`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Queues bytes for the player's UUID.

> Errors from the queue come back unchanged rather than wrapped or swallowed. The call
> needs a live callback's scope: outside one there is no queue to accept the bytes, and
> the call fails.

```rust
use pluto_lib::{Identifier, ServerPlayNetworking};
assert!(ServerPlayNetworking::send(7, &Identifier::new("example", "state").unwrap(), b"state").is_err());
```

### `ClientPlayNetworking`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Client receiver registration and sends to the connected server.

> Use `register_global_receiver` to hear the connected server's messages on a channel and
> `send` to queue bytes back to it. Sends run from a host-scoped callback; with no
> connection or no scope, `send` returns an error.

```rust
use pluto_lib::ClientPlayNetworking;
let _api = ClientPlayNetworking;
```

### `ClientPlayNetworking::register_global_receiver`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Takes a channel and a thread-safe static callback receiving borrowed server bytes.

> The callback must be thread-safe and static: a free function, or a closure that captures
> nothing by reference. The server's bytes are borrowed for the duration of the call, so
> copy anything you need to keep.

```rust
use pluto_lib::{Identifier, ClientPlayNetworking};
ClientPlayNetworking::register_global_receiver(Identifier::new("example", "state").unwrap(), |data| {
    let _received = data;
}).unwrap();
```

### `ClientPlayNetworking::send`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Queues bytes to a joined mod-capable server from a host-scoped callback.

> The call runs from a host-scoped callback; with no connection, or no scope, it returns
> an error instead of queueing. The bytes are queued for the connected server, not
> delivered synchronously.

```rust
use pluto_lib::{Identifier, ClientPlayNetworking};
assert!(ClientPlayNetworking::send(&Identifier::new("example", "request").unwrap(), b"request").is_err());
```

### `PlayerConnection`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: absent](badges/sunrise-absent.svg)

What a connection callback receives.

> `player` is the `ServerPlayer` (its methods work inside the callback), `name` is the
> player's name (also `player.name`).

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

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: absent](badges/sunrise-absent.svg)

The two connection events.

> The holder of the two connection events, JOIN and DISCONNECT. Callbacks registered on
> them hear when a player connects to the server and when they leave it.

```rust
use pluto_lib::ServerPlayConnectionEvents;

ServerPlayConnectionEvents::JOIN.register(|_| {});
ServerPlayConnectionEvents::DISCONNECT.register(|_| {});
```

### `ServerPlayConnectionEvents::JOIN`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: absent](badges/sunrise-absent.svg)

A player joined the server.

> The usual place to greet a player or give a starting item; the player handle works
> inside this callback.

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

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: absent](badges/sunrise-absent.svg)

A player left the server.

> The handle is still valid inside the callback, so last writes — saving data, clearing
> per-player state — happen here. After the callback returns the player is gone, and the
> handle must not be used.

```rust
use pluto_lib::{PlayerConnection, ServerPlayConnectionEvents, ServerPlayer};

ServerPlayConnectionEvents::DISCONNECT.register(|leave| println!("{} left", leave.name));
ServerPlayConnectionEvents::DISCONNECT.invoke(&PlayerConnection {
    player: ServerPlayer { uuid: 7, name: "Sam".into() },
    name: "Sam".into(),
});
```
