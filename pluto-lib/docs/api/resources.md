# Resource reloads

Read the active client resources and rebuild derived data when resources reload. Client reads follow the active resource-pack stack, rather than opening a file from a mod's source tree. Server-data notifications remain separate from client assets.

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

## Reading Pack-Aware Client Assets

Use a namespaced asset id: `tconstruct:textures/colormap/earth_grass_color.png` identifies `assets/tconstruct/textures/colormap/earth_grass_color.png` in the active client resources. `read_client_resource` returns the file's bytes; `read_client_image` returns decoded RGBA pixels. A resource-pack override therefore reaches a colour-map reader without hardcoding a repository or filesystem path.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType};

ResourceManagerHelper::get(ResourceType::ClientResources)
    .register_reload_listener(Identifier::new("example", "reload_colormap").unwrap(), || {
        let id = Identifier::new("tconstruct", "textures/colormap/earth_grass_color.png").unwrap();
        if let Some(image) = ResourceManagerHelper::read_client_image(&id) {
            assert_eq!(image.rgba.len() % 4, 0);
            assert_eq!(image.rgba.len() as u64 / 4, u64::from(image.width) * u64::from(image.height));
            // Replace the mod's cached colour map with these owned pixels.
        }
    });
```

Client-resource listeners receive an initial notification after the client backend is installed, then a notification after each successful client reload installs a fresh resource handle. Read again inside the listener; an old byte vector or image remains an owned snapshot and does not update itself. `ServerData` retains its initial-load and server `/reload` behaviour; it does not refresh client assets.

> **Use with Caution.** The native bridge has been built and checked in an integrated client: initial notification before join, one after actual F3+T, conflicting-pack priority, exact raw bytes/RGBA and unchanged old image snapshots. Reads return `None` without an installed client backend or when the resource cannot be read/decoded; a headless server or pure test must not assume an image exists. Triggering `reload_event(...).invoke(&())` only calls listeners: it does not reload packs or install a fresh handle. Dedicated/LAN and malformed native-pack reloads were not exercised.

## Reading JSON Data Folders

Register a folder under `data/<namespace>/`, not a filesystem path. The server backend discovers effective JSON resources across the resource root/content mods and enabled world datapacks; the top pack wins each resource ID. The callback receives decoded JSON, with the folder and `.json` suffix removed from each ID. Empty batches must clear data that disappeared.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType};
ResourceManagerHelper::get(ResourceType::ServerData)
    .register_json_reload_listener(Identifier::new("example", "rules").unwrap(), "custom/rules", |files| {
        // Build a new owned table from this complete batch, then replace the old one.
        for (id, json) in files { let _ = (id, json); }
        Ok(())
    }).unwrap();
```

> **Use with Caution.** Server folder discovery/apply is included in the v13 host/API/game-entry typechecks; executable linking, gameplay and combined attachment review remain pending. Native source sorts effective IDs and omits malformed JSON (including a broken top override, without reviving a lower pack). The facade validates the whole batch before invoking a callback outside registry locks. Callback errors are returned by pure dispatch and logged by the native bridge; they do not roll back other listeners or abort the whole server reload. There is no borrowed player/world context. `reload_registry` source-exposes owned entity-type, item and block membership/tag snapshots during these callbacks; retain that value when later work needs the same generation. Other registries remain unavailable. Folder callbacks run before `ServerData` notifications; notification phase ordering does not order folder callbacks. Registrations made during application affect the next native enumeration, not the current one. Client JSON declarations are accepted but currently inert, never redirected to server data.

## Reading Complete Resource Stacks

A tag loader needs every pack layer, not just the effective top JSON. During a server JSON callback, `reload_json_stacks` returns the complete stacks for any **exact directory captured in that reload pass**. Register every needed folder before the pass starts. All folders are prepared before any callback; reading another captured folder does not wait for its listener or depend on callback order. Each resource ID has bottom-to-top layers; malformed JSON and failed reads remain explicit errors at their original positions. This API does not merge tags, apply `replace`, skip errors or choose a fallback for you.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType};
let data = ResourceManagerHelper::get(ResourceType::ServerData);
data.register_json_reload_listener(Identifier::new("example", "tool_definitions").unwrap(),
    "tools/definitions", |_| Ok(())).unwrap();
data.register_json_reload_listener(Identifier::new("example", "material_tags").unwrap(), "tags/materials", |_| {
        let definitions = ResourceManagerHelper::reload_json_stacks("tools/definitions")
            .ok_or("definitions unavailable; do not mix with the previous generation")?;
        let _ = definitions; // captured before either listener, not loaded from a global cache
        let stacks = ResourceManagerHelper::reload_json_stacks("tags/materials")
            .ok_or("resource stacks unavailable")?;
        for stack in stacks {
            for layer in stack.layers {
                let json = layer?; // this strict example rejects any failed layer
                let _ = json; // a format-specific loader applies its own merge rules
            }
        }
        Ok(())
    }).unwrap();
```

> **Use with Caution.** Native source now scopes the whole reload-listener pass, preparing all registered folders from one pack stack before callbacks. Repeated directories share a capture; overlapping root/child directories reuse each pack/file's captured bytes. This is not an atomic filesystem transaction against external writers. Normal effective JSON and complete layers derive from those same bytes: a broken highest layer is omitted from the normal batch, never replaced by a lower pack, while the stack API retains both the valid lower layer and the error. This path is included in the v13 attachment plan and host/API/game-entry typechecks; executable linking and gameplay remain unverified. Production-helper regressions cover top-layer rejection and nested-scope restoration during unwinding; they do not exercise native pack I/O. `None` means unsupported/unavailable scope or an uncaptured/unsafe directory; `Some(empty)` means a successfully captured empty folder. Returned JSON is owned and survives reload/scope exit. Nested callbacks restore outer snapshots, including unwinding. Client JSON stacks remain unavailable. A loader that intentionally logs/skips a malformed layer must do so explicitly according to its source format; there is no implicit fallback.

## Capturing Reload Registry Membership

Inside a server JSON-folder callback, `reload_registry` returns an owned lookup for a supported registry. Native source uses the full entity-type ID data used by command validation, not the renderer-filtered MobTypes list. Entity tags come from the active world's successfully loaded pack-tag registry, with the native base-tag fallback only when that registry is unavailable. Item entries/tags use the world's currently selected native Items, not the fixed base Items or a built-in mod tag catalog. The fluid registry (`minecraft:fluid`) is the native fluid IDs joined with the fluids defined through [`FluidRegistry`](fluid.md), with the fluid tags read as the game reads its own (the resource root's `data/<namespace>/tags/fluid` and the world's pack stack, nested `#tag`s expanded); see [Fluid Tags](fluid.md#fluid-tags). The block registry (`minecraft:block`) is the world's own block registry — every block the active pack stack registers, vanilla and content alike — with the block tags of the world's pack-tag tables (the game's own tag read as fallback); tag members naming no registered block are dropped. An absent tag and a present empty tag remain distinct.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType};
ResourceManagerHelper::get(ResourceType::ServerData)
    .register_json_reload_listener(Identifier::new("example", "entity_rules").unwrap(), "entity_rules", |_| {
        let registry = Identifier::new("minecraft", "entity_type").unwrap();
        let entities = ResourceManagerHelper::reload_registry(&registry)
            .ok_or("entity registry unavailable; do not publish a fake empty table")?;
        let _player_exists = entities.contains(&Identifier::new("minecraft", "player").unwrap());
        Ok(())
    }).unwrap();
```

> **Use with Caution.** The native wrapper/provider is source-declared, guarded pending native attachment review, and not plan/type/build/game validated. `minecraft:entity_type`, `minecraft:item` and `minecraft:block` are source-supported. Outside the callback, or for unsupported/unavailable registries, the result is `None`, not a fabricated empty registry. Invalid registry contents are logged and make that registry unavailable without discarding valid independent registries. Broken or duplicate registry IDs invalidate the entire ambiguous context. Valid JSON is still delivered so a consumer can disable stale state. This does not abort or roll back the whole server reload. Owned lookups survive callback exit/reload, but retaining one does not refresh it. Entries/tag members are sets: no original registry order or seeded random-selection parity is promised. Nested callbacks restore the outer context, including panic unwinding; no world pointer or temporary entity ID is retained.

## Reference

### `ResourceType`

Which kind of resources a listener follows. `ClientResources` are client assets (models, textures, language files): listeners run once the initial client backend is installed and after successful client-resource reloads install their fresh handle. `ServerData` covers the server's data-table load/rebuild notification (recipes, loot tables, tags): initial world opening and subsequent `/reload`, through the pinned native server-data-loaded hook. Reloading one resource type does not invoke the other's listeners.

```rust
use pluto_lib::ResourceType;

assert_ne!(ResourceType::ClientResources, ResourceType::ServerData);
```

### `ResourceManagerHelper`

Registers reload listeners for one kind of resources and provides static read methods for the active client assets. Client resource ids are `namespace:path`, not paths into the working directory.

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

### `ResourceManagerHelper::register_json_reload_listener`

`fn register_json_reload_listener(&self, id: Identifier, directory: &str, apply: impl Fn(&[(Identifier, serde_json::Value)]) -> Result<(), String> + Send + Sync + 'static) -> Result<(), String>`. Registers a complete JSON-folder batch callback for this resource type. Duplicate listener IDs within the same type are rejected without replacement. Directories are nonempty slash-separated relative segments, without empty, `.` or `..` segments, backslashes or colons. Listener/resource IDs must also be plain namespaced paths. Callbacks may retain cloned JSON and register other listeners without a held registry lock. No client-native JSON discovery exists yet; server-native transport is source-wired but unverified.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType};
let data = ResourceManagerHelper::get(ResourceType::ServerData);
let id = Identifier::new("example", "json_data").unwrap();
data.register_json_reload_listener(id.clone(), "rules", |_| Ok(())).unwrap();
assert!(data.register_json_reload_listener(id, "other", |_| Ok(())).is_err());
assert!(data.register_json_reload_listener(Identifier::new("example", "unsafe").unwrap(), "../rules", |_| Ok(())).is_err());
```

### `ResourceManagerHelper::reload_json_stacks`

`fn reload_json_stacks(directory: &str) -> Option<Vec<JsonResourceStack>>`. Returns owned complete stacks for an exact directory captured in the current server reload pass, in backend resource-ID order, with layers bottom pack first. All registered folders are captured before callbacks; another captured folder is readable regardless of listener order. Unsafe, uncaptured or unavailable directories and out-of-scope/client backends return `None`. A captured folder with no resources returns `Some(vec![])`. This reads the captured snapshot, not the filesystem again.

```rust
use pluto_lib::ResourceManagerHelper;
assert!(ResourceManagerHelper::reload_json_stacks("tags/materials").is_none()); // outside callback
assert!(ResourceManagerHelper::reload_json_stacks("../escape").is_none());
```

### `JsonResourceStack`

Owned JSON layers: `id: Identifier` is the resource ID with the directory and `.json` suffix removed; `layers: Vec<Result<serde_json::Value, String>>` retains each pack's JSON or read/parse error, bottom-to-top. Error strings identify their resource/layer where available. Cloning or editing this value does not change the callback's retained snapshot. Manual construction does not install packs or publish a reload.

```rust
use pluto_lib::{Identifier, JsonResourceStack};
let stack = JsonResourceStack {
    id: Identifier::new("example", "ores").unwrap(),
    layers: vec![Ok(serde_json::json!({"values": ["example:copper"]})), Err("invalid top JSON".into())],
};
assert_eq!(stack.id.to_string(), "example:ores");
assert!(stack.layers[0].is_ok());
assert!(stack.layers[1].is_err()); // not replaced by the lower layer
```

### `ResourceManagerHelper::reload_registry`

`fn reload_registry(registry: &Identifier) -> Option<RegistryLookup>`. Captures a supported registry's membership/tags during a server JSON callback. Outside that scope, and for unsupported or unavailable registries, returns `None`. Source support currently covers `minecraft:entity_type`, `minecraft:item`, `minecraft:fluid` (membership with mod fluids and tags) and `minecraft:block` (the world's blocks and their tags); native integration remains unverified.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper};
for registry in ["entity_type", "item"] {
    let id = Identifier::new("minecraft", registry).unwrap();
    assert!(ResourceManagerHelper::reload_registry(&id).is_none()); // outside a reload callback
}
```

### `RegistryLookup`

A cloneable, owned registry/tag membership snapshot. Keep it to pair derived data with the same reload generation. Values cannot be edited through this view; retaining or cloning one does not refresh it.

```rust
use pluto_lib::{Identifier, RegistryLookup, ResourceManagerHelper};
let snapshot: Option<RegistryLookup> = ResourceManagerHelper::reload_registry(&Identifier::parse("minecraft:entity_type").unwrap());
let retained = snapshot.clone();
assert_eq!(snapshot, retained);
```

### `RegistryLookup::contains`

`fn contains(&self, id: &Identifier) -> bool`. Tests actual captured registry membership, independent of renderer/model availability.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper};
if let Some(entities) = ResourceManagerHelper::reload_registry(&Identifier::parse("minecraft:entity_type").unwrap()) {
    let _exists = entities.contains(&Identifier::parse("minecraft:player").unwrap());
}
```

### `RegistryLookup::get_tag`

`fn get_tag(&self, tag: &Identifier) -> Option<&std::collections::BTreeSet<Identifier>>`. Returns the captured tag's members; `None` means absent, `Some(empty)` means present but empty. This is membership, not an ordered tag RNG API.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper};
if let Some(entities) = ResourceManagerHelper::reload_registry(&Identifier::parse("minecraft:entity_type").unwrap()) {
    if let Some(members) = entities.get_tag(&Identifier::parse("minecraft:skeletons").unwrap()) {
        let _has_skeleton = members.contains(&Identifier::parse("minecraft:skeleton").unwrap());
    }
}
```

### `RegistryLookup::tags`

`fn tags(&self) -> &std::collections::BTreeMap<Identifier, std::collections::BTreeSet<Identifier>>`. Every captured tag with its members, for consumers that republish the resolved snapshot (the advancement tag overlay). A tag the reload did not capture is not in this map.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper};
if let Some(items) = ResourceManagerHelper::reload_registry(&Identifier::parse("minecraft:item").unwrap()) {
    for (tag, members) in items.tags() {
        let _count = (tag, members.len());
    }
}
```

### `RegistryLookup::with_extra_tags`

`fn with_extra_tags(&self, extra: &[(Identifier, Vec<Identifier>)]) -> RegistryLookup`. A copy whose tags also hold the given members (a mod that needs a convention tag such as `forge:ingots/iron` that nothing in the game defines). Members that are not registry entries are dropped, and a tag with no remaining member is not created. The original snapshot is unchanged; redo it for every reload.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper};
if let Some(items) = ResourceManagerHelper::reload_registry(&Identifier::parse("minecraft:item").unwrap()) {
    let tag = Identifier::parse("forge:ingots/iron").unwrap();
    let with = items.with_extra_tags(&[(tag.clone(), vec![Identifier::parse("minecraft:iron_ingot").unwrap()])]);
    let _held = with.get_tag(&tag);
}
```

### `ResourceManagerHelper::read_client_resource`

`fn read_client_resource(id: &Identifier) -> Option<Vec<u8>>`. Read the active client asset identified by `id`, respecting the resource-pack stack. The returned bytes are owned; `None` means no client backend is installed, the id contains a non-plain path (empty, `.` or `..` segments), or the asset could not be read. This does not read server datapacks, and does not validate the file's format.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper};

let id = Identifier::new("tconstruct", "textures/colormap/earth_grass_color.png").unwrap();
if let Some(bytes) = ResourceManagerHelper::read_client_resource(&id) {
    assert!(!bytes.is_empty()); // a present colour-map PNG has file bytes
}
// A pure test without a client backend may receive None.
```

### `ResourceManagerHelper::read_client_image`

`fn read_client_image(id: &Identifier) -> Option<ResourceImage>`. Read and decode a PNG from the active client resources. The path must start with `textures/` and end in `.png`, for example `tconstruct:textures/colormap/earth_grass_color.png`. Pixels are top-row-first, gamma-encoded, non-premultiplied RGBA bytes; each pixel occupies four consecutive bytes. `None` covers an unsupported/non-plain id, missing client backend, missing/unreadable resource, decode failure, zero dimensions or an inconsistent pixel-buffer length.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper};

let id = Identifier::new("tconstruct", "textures/colormap/earth_grass_color.png").unwrap();
if let Some(image) = ResourceManagerHelper::read_client_image(&id) {
    assert!(image.width > 0 && image.height > 0);
    assert_eq!(image.rgba.len() % 4, 0);
    assert_eq!(image.rgba.len() as u64 / 4, u64::from(image.width) * u64::from(image.height));
}
// No image is required in the host-independent doctest environment.
```

> **Use with Caution.** `read_client_image` explicitly returns `None` for the `horizon_cit` namespace: Horizon's texture decoder treats it as an OptiFine-specific path, not the ordinary `assets/horizon_cit/` namespace. The facade does not silently redirect an image read or install a separate decoder. `read_client_resource` can still read raw bytes from the normal `assets/horizon_cit/<path>` location when that resource exists.

### `ResourceImage`

An owned decoded image: `width: u32` and `height: u32` count pixels; `rgba: Vec<u8>` stores top-row-first, gamma-encoded, non-premultiplied red/green/blue/alpha bytes. For a valid image the vector contains `width * height` pixels, four bytes each. A copied image is a snapshot, not a live view of future pack reloads. When constructing one manually, keep its dimensions and buffer consistent.

```rust
use pluto_lib::ResourceImage;

let image = ResourceImage {
    width: 2,
    height: 1,
    rgba: vec![255, 0, 0, 255, 0, 128, 255, 128],
};
assert_eq!((image.width, image.height), (2, 1));
assert_eq!(&image.rgba[4..8], &[0, 128, 255, 128]); // RGB is not scaled by alpha
```

#### Image width

`width: u32` is the number of pixels in each row.

```rust
use pluto_lib::ResourceImage;
let image = ResourceImage { width: 2, height: 1, rgba: vec![0; 8] };
assert_eq!(image.width, 2);
```

#### Image height

`height: u32` is the number of rows, starting with the top row.

```rust
use pluto_lib::ResourceImage;
let image = ResourceImage { width: 1, height: 2, rgba: vec![255, 0, 0, 255, 0, 0, 255, 255] };
assert_eq!(image.height, 2);
assert_eq!(&image.rgba[..4], &[255, 0, 0, 255]); // top row
```

#### RGBA pixels

`rgba: Vec<u8>` contains the four unpremultiplied channels of each pixel in row-major order. Check bounds before indexing; the byte offset of pixel `(x, y)` is `(y * width + x) * 4`.

```rust
use pluto_lib::ResourceImage;
let image = ResourceImage { width: 1, height: 1, rgba: vec![80, 160, 240, 128] };
assert_eq!(&image.rgba[..], &[80, 160, 240, 128]);
assert_eq!(image.rgba[3], 128);
```

### `reload_event`

`fn reload_event(resource_type: ResourceType) -> &'static Event<(), ()>`. The event behind the listeners of a kind of resources; use it to order listeners or, in tests, to trigger a reload.

```rust
use pluto_lib::{reload_event, Identifier, ResourceType};

let first = Identifier::new("example", "first").unwrap();
let second = Identifier::new("example", "second").unwrap();
reload_event(ResourceType::ClientResources).add_phase_ordering(first, second).unwrap();
```
