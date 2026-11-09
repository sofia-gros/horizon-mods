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

> **Use with Caution.** The new client-read/initial-notification bridge is awaiting local build and game validation in this slice. Reads return `None` without an installed client backend or when the resource cannot be read/decoded, so a headless server or a pure test must not assume an image exists. Triggering `reload_event(...).invoke(&())` in a test only calls listeners; it does not reload packs or install a fresh client handle.

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

### `ResourceManagerHelper::read_client_resource`

`fn read_client_resource(id: &Identifier) -> Option<Vec<u8>>`. Read the active client asset identified by `id`, respecting the resource-pack stack. The returned bytes are owned; `None` means no client backend is installed or the asset could not be read. This does not read server datapacks, and does not validate the file's format.

```rust
use pluto_lib::{Identifier, ResourceManagerHelper};

let id = Identifier::new("tconstruct", "textures/colormap/earth_grass_color.png").unwrap();
if let Some(bytes) = ResourceManagerHelper::read_client_resource(&id) {
    assert!(!bytes.is_empty()); // a present colour-map PNG has file bytes
}
// A pure test without a client backend may receive None.
```

### `ResourceManagerHelper::read_client_image`

`fn read_client_image(id: &Identifier) -> Option<ResourceImage>`. Read and decode an image from the active client resources. The `id` names the asset file, for example `tconstruct:textures/colormap/earth_grass_color.png`. Pixels are top-row-first, non-premultiplied RGBA bytes; each pixel occupies four consecutive bytes. `None` covers a missing client backend, missing/unreadable resource or an image that cannot be decoded.

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

### `ResourceImage`

An owned decoded image: `width: u32` and `height: u32` count pixels; `rgba: Vec<u8>` stores top-row-first, non-premultiplied red/green/blue/alpha bytes. For a valid image the vector contains `width * height` pixels, four bytes each. A copied image is a snapshot, not a live view of future pack reloads. When constructing one manually, keep its dimensions and buffer consistent.

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
