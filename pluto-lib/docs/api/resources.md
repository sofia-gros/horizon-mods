# Resource reloads

Be told after resources reload, to rebuild whatever a mod derived from them.

### `ResourceType`

Which kind of resources a listener follows. `ClientResources` are assets (models, textures, language files) and work. `ServerData` is data (recipes, loot tables, tags) and is a stub: listeners are accepted and not invoked yet (support matrix).

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
