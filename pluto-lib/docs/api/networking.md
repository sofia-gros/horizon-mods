# Networking: player connections

Be told when a player joins or leaves. Callbacks get a `PlayerConnection` whose `player` can be used right away to message or give items (see `player.md`).

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
