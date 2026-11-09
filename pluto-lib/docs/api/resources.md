# Resource reloads

Be told after resources reload, to rebuild whatever a mod derived from them.

## Basic Example

Rebuild something you derived from the resources after every reload:

```rust
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType};

ResourceManagerHelper::get(ResourceType::ClientResources)
    .register_reload_listener(Identifier::new("example", "rebuild_cache").unwrap(), || {
        // models, textures or language files changed: rebuild
    });
```

## Order Between Listeners

Each listener has an id, which is its phase, so two listeners can be ordered with `reload_event`: the one that builds a table before the one that reads it.

```rust
use pluto_lib::{reload_event, Identifier, ResourceType};

let tables = Identifier::new("example", "tables").unwrap();
let users = Identifier::new("example", "users").unwrap();
reload_event(ResourceType::ClientResources).add_phase_ordering(tables, users).unwrap();
```

> **Use with Caution.** `ResourceType::ServerData` runs after the server's initial data load (before chunks and players) and after each `/reload` rebuild, when the new tables are available. It is a server notification, not a client-assets reload, and provides no live player scope. Keep listeners suitable for both initial load and reload; use phase ordering for dependencies.

## Reference

### `ResourceType`

Which kind of resources a listener follows. `ClientResources` are client assets (models, textures, language files). `ServerData` covers the server's data-table load/rebuild notification (recipes, loot tables, tags): initial world opening and subsequent `/reload`, through the pinned native server-data-loaded hook. Reloading one resource type does not invoke the other's listeners.

```rust
use pluto_lib::ResourceType;

assert_ne!(ResourceType::ClientResources, ResourceType::ServerData);
```

### `ResourceManagerHelper`

Registers reload listeners for one kind of resources.

```rust
use pluto_lib::{ResourceManagerHelper, ResourceType};

let helper = ResourceManagerHelper::get(ResourceType::ClientResources);
helper.register_reload_listener(pluto_lib::Identifier::new("example", "rebuild").unwrap(), || {});
```

### `ResourceManagerHelper::get`

`fn get(resource_type: ResourceType) -> ResourceManagerHelper`.

```rust
use pluto_lib::{ResourceManagerHelper, ResourceType};

let assets = ResourceManagerHelper::get(ResourceType::ClientResources);
let data = ResourceManagerHelper::get(ResourceType::ServerData);
let _ = (assets, data);
```

### `ResourceManagerHelper::register_reload_listener`

`fn register_reload_listener(&self, id: Identifier, listener: impl Fn() + Send + Sync + 'static)`. The listener runs after every reload of this kind of resources. The identifier names its phase, so listeners can be ordered with `Event::add_phase_ordering` on `reload_event`.

```rust
use pluto_lib::{reload_event, Identifier, ResourceManagerHelper, ResourceType};
use std::sync::atomic::{AtomicU32, Ordering};

static RELOADS: AtomicU32 = AtomicU32::new(0);
ResourceManagerHelper::get(ResourceType::ClientResources)
    .register_reload_listener(Identifier::new("example", "count").unwrap(), || {
        RELOADS.fetch_add(1, Ordering::Relaxed);
    });
reload_event(ResourceType::ClientResources).invoke(&());
assert_eq!(RELOADS.load(Ordering::Relaxed), 1);
```

### `reload_event`

`fn reload_event(resource_type: ResourceType) -> &'static Event<(), ()>`. The event behind the listeners of a kind of resources; use it to order listeners or, in tests, to trigger a reload.

```rust
use pluto_lib::{reload_event, Identifier, ResourceType};

let first = Identifier::new("example", "first").unwrap();
let second = Identifier::new("example", "second").unwrap();
reload_event(ResourceType::ClientResources).add_phase_ordering(first, second).unwrap();
```
