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

> **Use with Caution.** `SERVER_STARTED` is called at the first server tick after the server started, because Horizon has no earlier point. `SERVER_STOPPED` runs after the integrated or dedicated server's successful final save and teardown, through PlutoLib's pinned source Mixins. A failed save does not report successful completion. No live player operations are available there.

## Reference

## Data

### `ServerInfo`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

What the server callbacks learn about the server.

> Its field `dedicated` is true for a dedicated server and false for the integrated one —
> single player or a LAN host. Branch on it for setup that only makes sense on one kind of
> server.

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

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

A world (dimension) tick.

> Its field `dimension` is the dimension id, such as `minecraft:overworld`, naming the
> world that is starting or ending this tick.

```rust
use pluto_lib::{ServerTickEvents, WorldTick};

ServerTickEvents::END_WORLD_TICK.register(|tick: &WorldTick| {
    assert_eq!(tick.dimension, "minecraft:the_nether");
});
ServerTickEvents::END_WORLD_TICK.invoke(&WorldTick { dimension: "minecraft:the_nether".to_owned() });
```

### `ServerChunk`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

A chunk of a world, as chunk callbacks receive it.

> `dimension` is the dimension id, `x` and `z` are the chunk coordinates, and
> `newly_generated` is true on `CHUNK_LOAD` when the chunk was generated just now rather
> than read from disk. Multiply `x` and `z` by 16 for block coordinates.

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

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

The four server lifecycle events.

> `SERVER_STARTING` and `SERVER_STOPPING` run before startup and teardown;
> `SERVER_STARTED` is invoked at the first server tick after startup. `SERVER_STOPPED`
> runs once after a successful final save and teardown, and is not emitted when the stop
> routine returns a save or teardown error.

```rust
use pluto_lib::ServerLifecycleEvents;

ServerLifecycleEvents::SERVER_STARTING.register(|_| {});
ServerLifecycleEvents::SERVER_STOPPING.register(|_| {});
```

### `ServerLifecycleEvents::SERVER_STARTING`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

The server is about to start, before the world loads.

> Use it to read configuration or register data. The callback receives the `ServerInfo`,
> so it can branch on whether the server is dedicated.

```rust
use pluto_lib::{ServerInfo, ServerLifecycleEvents};

ServerLifecycleEvents::SERVER_STARTING.register(|server| println!("starting, dedicated: {}", server.dedicated));
ServerLifecycleEvents::SERVER_STARTING.invoke(&ServerInfo { dedicated: false });
```

### `ServerLifecycleEvents::SERVER_STARTED`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

The server has finished starting.

> It is called at the first server tick after the server started, so by the time callbacks
> run the world is up. The callback receives the `ServerInfo`.

```rust
use pluto_lib::ServerLifecycleEvents;

ServerLifecycleEvents::SERVER_STARTED.register(|server| println!("started, dedicated={}", server.dedicated));
```

### `ServerLifecycleEvents::SERVER_STOPPING`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

The server is about to stop.

> Use it to save your own data while the server is still running. The callback receives
> the `ServerInfo`.

```rust
use pluto_lib::{ServerInfo, ServerLifecycleEvents};

ServerLifecycleEvents::SERVER_STOPPING.register(|_| { /* flush caches */ });
ServerLifecycleEvents::SERVER_STOPPING.invoke(&ServerInfo { dedicated: true });
```

### `ServerLifecycleEvents::SERVER_STOPPED`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

The server has finished its successful final save and teardown.

> The callback receives `ServerInfo { dedicated }`; use it to release mod-owned resources,
> not to operate on departed players. A crash or failed final save is not reported as a
> successful shutdown.

```rust
use pluto_lib::ServerLifecycleEvents;

ServerLifecycleEvents::SERVER_STOPPED.register(|_| {});
```

## Ticks

### `ServerTickEvents`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

Tick events, each running every game tick.

> Its fields are the start and end events for the server tick and for each world tick:
> `START_SERVER_TICK`, `END_SERVER_TICK`, `START_WORLD_TICK` and `END_WORLD_TICK`. Keep
> callbacks short — they run twenty times a second, so a little work each time adds up.

```rust
use pluto_lib::ServerTickEvents;

ServerTickEvents::START_SERVER_TICK.register(|_| {});
ServerTickEvents::END_SERVER_TICK.register(|_| {});
```

### `ServerTickEvents::START_SERVER_TICK`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

Before the server runs a tick.

> The callback receives no value. Pair it with `END_SERVER_TICK` to bracket a tick, and
> keep the work small enough to run twenty times a second.

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

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

After the server has run a tick.

> The callback receives no value. It is the usual place for once-a-tick bookkeeping;
> prefer a counter and do real work only every few ticks.

```rust
use pluto_lib::ServerTickEvents;

ServerTickEvents::END_SERVER_TICK.register(|_| { /* once per tick */ });
ServerTickEvents::END_SERVER_TICK.invoke(&());
```

### `ServerTickEvents::START_WORLD_TICK`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

Before a world (dimension) ticks.

> The callback receives the `WorldTick`, whose `dimension` names the world about to tick.

```rust
use pluto_lib::{ServerTickEvents, WorldTick};

ServerTickEvents::START_WORLD_TICK.register(|world| assert!(world.dimension.contains(':')));
ServerTickEvents::START_WORLD_TICK.invoke(&WorldTick { dimension: "minecraft:overworld".into() });
```

### `ServerTickEvents::END_WORLD_TICK`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

After a world (dimension) ticked.

> The callback receives the `WorldTick`, whose `dimension` names the world that just
> ticked.

```rust
use pluto_lib::{ServerTickEvents, WorldTick};

ServerTickEvents::END_WORLD_TICK.register(|world| {
    let _ = &world.dimension;
});
ServerTickEvents::END_WORLD_TICK.invoke(&WorldTick { dimension: "minecraft:overworld".into() });
```

## Chunks

### `ServerChunkEvents`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

The holder of the chunk load and unload events.

> Callbacks registered on its fields — `CHUNK_LOAD` and `CHUNK_UNLOAD` — hear when a chunk
> loads into a world and when it unloads, which is where per-chunk setup and teardown
> belong.

```rust
use pluto_lib::ServerChunkEvents;

ServerChunkEvents::CHUNK_LOAD.register(|_| {});
ServerChunkEvents::CHUNK_UNLOAD.register(|_| {});
```

### `ServerChunkEvents::CHUNK_LOAD`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

A chunk loaded.

> Check `newly_generated` on the `ServerChunk` to tell fresh terrain from a chunk read
> from disk, and read its `dimension`, `x` and `z` for where it loaded.

```rust
use pluto_lib::{ServerChunk, ServerChunkEvents};

ServerChunkEvents::CHUNK_LOAD.register(|chunk| assert_eq!((chunk.x, chunk.z), (3, -2)));
ServerChunkEvents::CHUNK_LOAD.invoke(&ServerChunk { dimension: "minecraft:overworld".into(), x: 3, z: -2, newly_generated: false });
```

### `ServerChunkEvents::CHUNK_UNLOAD`

![Horizon: unclassified](badges/horizon-unclassified.svg) ![Sunrise: unclassified](badges/sunrise-unclassified.svg)

A chunk unloaded.

> `newly_generated` on the `ServerChunk` is false here; its `dimension`, `x` and `z`
> fields say which chunk went away. Use it to drop any per-chunk state you kept for it.

```rust
use pluto_lib::{ServerChunk, ServerChunkEvents};

ServerChunkEvents::CHUNK_UNLOAD.register(|chunk| assert!(!chunk.newly_generated));
ServerChunkEvents::CHUNK_UNLOAD.invoke(&ServerChunk { dimension: "minecraft:overworld".into(), x: 0, z: 0, newly_generated: false });
```
