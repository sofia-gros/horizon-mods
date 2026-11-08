# Lifecycle events: server, world, chunk

Be told when the server starts and stops, when each tick begins and ends, and when chunks load. Callbacks receive small owned values (not game objects); to drive a callback in a test, build the value and call `invoke`.

## Basic Example

Run code when the server starts and stops:

```rust
use pluto_lib::ServerLifecycleEvents;

ServerLifecycleEvents::SERVER_STARTING.register(|server| {
    println!("starting a {} server", if server.dedicated { "dedicated" } else { "integrated" });
});
ServerLifecycleEvents::SERVER_STOPPING.register(|_| { /* save your own data */ });
```

## Doing Something Every Tick

The tick events run twenty times a second, so a callback that does a little work each time adds up:

```rust
use pluto_lib::ServerTickEvents;
use std::sync::atomic::{AtomicU64, Ordering};

static TICKS: AtomicU64 = AtomicU64::new(0);
ServerTickEvents::END_SERVER_TICK.register(|_| {
    if TICKS.fetch_add(1, Ordering::Relaxed) % 20 == 0 {
        // once a second
    }
});
```

> **Tip.** Prefer a counter and do real work every few ticks. For work tied to one block, use a block entity ticker instead (see [block-entity](block-entity.md)).

## Chunks

`CHUNK_LOAD` and `CHUNK_UNLOAD` carry the dimension and chunk coordinates; on load, `newly_generated` tells fresh terrain from a chunk read from disk.

```rust
use pluto_lib::ServerChunkEvents;

ServerChunkEvents::CHUNK_LOAD.register(|chunk| {
    if chunk.newly_generated {
        // decorate new terrain
    }
});
```

> **Use with Caution.** `SERVER_STARTED` and `SERVER_STOPPED` are accepted but not called yet (support matrix).

## Reference

## Data

### `ServerInfo`

What the server callbacks learn about the server. `dedicated` is true for a dedicated server and false for the integrated one (single player or LAN host).

```rust
use pluto_lib::{ServerInfo, ServerLifecycleEvents};

ServerLifecycleEvents::SERVER_STARTING.register(|server: &ServerInfo| {
    if server.dedicated {
        // dedicated-server-only setup
    }
});
ServerLifecycleEvents::SERVER_STARTING.invoke(&ServerInfo { dedicated: true });
```

### `WorldTick`

A world (dimension) tick. `dimension` is the dimension id, such as `minecraft:overworld`.

```rust
use pluto_lib::{ServerTickEvents, WorldTick};

ServerTickEvents::END_WORLD_TICK.register(|tick: &WorldTick| {
    assert_eq!(tick.dimension, "minecraft:the_nether");
});
ServerTickEvents::END_WORLD_TICK.invoke(&WorldTick { dimension: "minecraft:the_nether".to_owned() });
```

### `ServerChunk`

A chunk of a world. `dimension` is the dimension id, `x` and `z` are the chunk coordinates, and `newly_generated` is true on `CHUNK_LOAD` when the chunk was generated just now rather than read from disk.

```rust
use pluto_lib::{ServerChunk, ServerChunkEvents};

ServerChunkEvents::CHUNK_LOAD.register(|chunk: &ServerChunk| {
    if chunk.newly_generated && chunk.dimension == "minecraft:overworld" {
        let _block_x = chunk.x * 16;
        let _block_z = chunk.z * 16;
    }
});
```

## Server lifecycle

### `ServerLifecycleEvents`

The four server lifecycle events. `SERVER_STARTING` and `SERVER_STOPPING` work. `SERVER_STARTED` and `SERVER_STOPPED` are accepted but never invoked yet (see the support matrix: Horizon has no matching hook point).

```rust
use pluto_lib::ServerLifecycleEvents;

ServerLifecycleEvents::SERVER_STARTING.register(|_| {});
ServerLifecycleEvents::SERVER_STOPPING.register(|_| {});
```

### `ServerLifecycleEvents::SERVER_STARTING`

The server is about to start (before the world loads). Use it to read configuration or register data.

```rust
use pluto_lib::{ServerInfo, ServerLifecycleEvents};

ServerLifecycleEvents::SERVER_STARTING.register(|server| println!("starting, dedicated: {}", server.dedicated));
ServerLifecycleEvents::SERVER_STARTING.invoke(&ServerInfo { dedicated: false });
```

### `ServerLifecycleEvents::SERVER_STARTED`

Stub: the server has finished starting. Callbacks are accepted and not invoked yet.

```rust
use pluto_lib::ServerLifecycleEvents;

ServerLifecycleEvents::SERVER_STARTED.register(|_| {}); // accepted; runs once Horizon reports it
```

### `ServerLifecycleEvents::SERVER_STOPPING`

The server is about to stop. Use it to save your own data.

```rust
use pluto_lib::{ServerInfo, ServerLifecycleEvents};

ServerLifecycleEvents::SERVER_STOPPING.register(|_| { /* flush caches */ });
ServerLifecycleEvents::SERVER_STOPPING.invoke(&ServerInfo { dedicated: true });
```

### `ServerLifecycleEvents::SERVER_STOPPED`

Stub: the server has finished stopping. Callbacks are accepted and not invoked yet.

```rust
use pluto_lib::ServerLifecycleEvents;

ServerLifecycleEvents::SERVER_STOPPED.register(|_| {});
```

## Ticks

### `ServerTickEvents`

Tick events, each running every game tick. Keep callbacks short.

```rust
use pluto_lib::ServerTickEvents;

ServerTickEvents::START_SERVER_TICK.register(|_| {});
ServerTickEvents::END_SERVER_TICK.register(|_| {});
```

### `ServerTickEvents::START_SERVER_TICK`

Before the server runs a tick.

```rust
use pluto_lib::ServerTickEvents;
use std::sync::atomic::{AtomicU64, Ordering};

static TICKS: AtomicU64 = AtomicU64::new(0);
ServerTickEvents::START_SERVER_TICK.register(|_| {
    TICKS.fetch_add(1, Ordering::Relaxed);
});
ServerTickEvents::START_SERVER_TICK.invoke(&());
assert_eq!(TICKS.load(Ordering::Relaxed), 1);
```

### `ServerTickEvents::END_SERVER_TICK`

After the server has run a tick.

```rust
use pluto_lib::ServerTickEvents;

ServerTickEvents::END_SERVER_TICK.register(|_| { /* once per tick */ });
ServerTickEvents::END_SERVER_TICK.invoke(&());
```

### `ServerTickEvents::START_WORLD_TICK`

Before a world (dimension) ticks.

```rust
use pluto_lib::{ServerTickEvents, WorldTick};

ServerTickEvents::START_WORLD_TICK.register(|world| assert!(world.dimension.contains(':')));
ServerTickEvents::START_WORLD_TICK.invoke(&WorldTick { dimension: "minecraft:overworld".into() });
```

### `ServerTickEvents::END_WORLD_TICK`

After a world (dimension) ticked.

```rust
use pluto_lib::{ServerTickEvents, WorldTick};

ServerTickEvents::END_WORLD_TICK.register(|world| {
    let _ = &world.dimension;
});
ServerTickEvents::END_WORLD_TICK.invoke(&WorldTick { dimension: "minecraft:overworld".into() });
```

## Chunks

### `ServerChunkEvents`

Chunk load and unload events.

```rust
use pluto_lib::ServerChunkEvents;

ServerChunkEvents::CHUNK_LOAD.register(|_| {});
ServerChunkEvents::CHUNK_UNLOAD.register(|_| {});
```

### `ServerChunkEvents::CHUNK_LOAD`

A chunk loaded. Check `newly_generated` to tell fresh terrain from a chunk read from disk.

```rust
use pluto_lib::{ServerChunk, ServerChunkEvents};

ServerChunkEvents::CHUNK_LOAD.register(|chunk| assert_eq!((chunk.x, chunk.z), (3, -2)));
ServerChunkEvents::CHUNK_LOAD.invoke(&ServerChunk { dimension: "minecraft:overworld".into(), x: 3, z: -2, newly_generated: false });
```

### `ServerChunkEvents::CHUNK_UNLOAD`

A chunk unloaded. `newly_generated` is false.

```rust
use pluto_lib::{ServerChunk, ServerChunkEvents};

ServerChunkEvents::CHUNK_UNLOAD.register(|chunk| assert!(!chunk.newly_generated));
ServerChunkEvents::CHUNK_UNLOAD.invoke(&ServerChunk { dimension: "minecraft:overworld".into(), x: 0, z: 0, newly_generated: false });
```
