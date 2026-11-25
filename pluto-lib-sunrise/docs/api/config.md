# Config files

PlutoLib extra (Fabric API has none; NeoForge has `ModConfigEvent`). Register a config file with its default text, then hear when its text is loaded, changed on disk or dropped. Parsing is the mod's business (TOML by convention), so the API deals in text.

Register inside your mod's `hooks()`, before the game asks which configs exist.

## Basic Example

Ask for a config file with its default text; hear when it is loaded. Parsing is yours (TOML by convention):

```rust
use pluto_lib::{ConfigEvents, ConfigKind, ConfigRegistry};
use std::sync::atomic::{AtomicBool, Ordering};

static GREET: AtomicBool = AtomicBool::new(true);

ConfigRegistry::register("example", ConfigKind::Common, "greet = true\n");
ConfigEvents::LOADING.register(|config| {
    if config.mod_id == "example" {
        GREET.store(!config.text.contains("greet = false"), Ordering::Relaxed);
    }
});
```

The file is `config/example-common.toml`, created from the default text when it is missing.

## Which Kind

`ConfigKind::Client` is read by the window when the game starts, `Common` once per process (the window or a dedicated server), `Server` for each world the server opens. Put a setting where it is used: a drawing option is `Client`, a gameplay rule that every player must share is `Server`.

## Changing the File While Playing

The game looks at the file once a second. When the text changes, `ConfigEvents::RELOADING` fires with the new text, and `UNLOADING` fires when a `Server` config is dropped as its world closes:

```rust
use pluto_lib::ConfigEvents;

ConfigEvents::RELOADING.register(|config| {
    if config.mod_id == "example" {
        println!("{} changed", config.file);
    }
});
```

> **Tip.** Every mod hears every config, so always check `config.mod_id` first. A file saved again with the same text fires nothing.

## Reference

### `ConfigKind`

Which side reads a config and when. `Client` loads on the window when the game starts, `Common` once per process (the window or a dedicated server), `Server` for each world the server opens.

```rust
use pluto_lib::ConfigKind;

let kinds = [ConfigKind::Client, ConfigKind::Common, ConfigKind::Server];
assert_eq!(kinds.len(), 3);
```

### `ConfigRegistry`

Registers config files.

```rust
use pluto_lib::{ConfigKind, ConfigRegistry};

ConfigRegistry::register("example", ConfigKind::Common, "greeting = \"hello\"\n");
```

### `ConfigRegistry::register`

`fn register(mod_id: &str, kind: ConfigKind, default_text: &str)`. Asks for `config/<mod_id>-<kind>.toml` (`example-common.toml` above), created from `default_text` when missing.

```rust
use pluto_lib::{ConfigKind, ConfigRegistry};

ConfigRegistry::register("example", ConfigKind::Server, "difficulty = 2\n");
```

### `ConfigRegistry::register_file`

`fn register_file(mod_id: &str, kind: ConfigKind, file: &str, default_text: &str)`. The same in another file inside `config/`, such as `example/machines.toml`.

```rust
use pluto_lib::{ConfigKind, ConfigRegistry};

ConfigRegistry::register_file("example", ConfigKind::Common, "example/machines.toml", "[machines]\nspeed = 1\n");
```

### `ConfigSpec`

A config as registered. `mod_id` is the owner, `kind` when it loads, `file` the file name inside `config/` if it is not the default one, `default_text` what is written when the file is missing. PlutoLib builds these from `register` and `register_file`; read them back with a `ConfigSpec` only in tooling.

```rust
use pluto_lib::{ConfigKind, ConfigSpec};

let spec = ConfigSpec { mod_id: "example".into(), kind: ConfigKind::Common, file: None, default_text: String::new() };
assert!(spec.file.is_none());
assert_eq!(spec.mod_id, "example");
assert_eq!(spec.kind, ConfigKind::Common);
assert!(spec.default_text.is_empty());
```

### `ModConfig`

A config file as the game read it. `mod_id` is the owner, `kind` its kind, `file` the name inside `config/`, `text` the file's contents.

```rust
use pluto_lib::{ConfigEvents, ConfigKind, ModConfig};

ConfigEvents::LOADING.register(|config: &ModConfig| {
    if config.mod_id == "example" {
        assert!(config.text.contains("greeting"));
    }
});
ConfigEvents::LOADING.invoke(&ModConfig {
    mod_id: "example".into(),
    kind: ConfigKind::Common,
    file: "example-common.toml".into(),
    text: "greeting = \"hello\"\n".into(),
});
```

### `ConfigEvents`

The three config events. Every mod hears every config, so a callback checks `config.mod_id`.

```rust
use pluto_lib::ConfigEvents;

ConfigEvents::LOADING.register(|_| {});
ConfigEvents::RELOADING.register(|_| {});
ConfigEvents::UNLOADING.register(|_| {});
```

### `ConfigEvents::LOADING`

A registered config was read. Parse `config.text` here.

```rust
use pluto_lib::{ConfigEvents, ConfigKind, ModConfig};
use std::sync::atomic::{AtomicBool, Ordering};

static GREETING_ON: AtomicBool = AtomicBool::new(true);
ConfigEvents::LOADING.register(|config| {
    if config.mod_id == "example" {
        GREETING_ON.store(!config.text.contains("greeting = false"), Ordering::Relaxed);
    }
});
ConfigEvents::LOADING.invoke(&ModConfig { mod_id: "example".into(), kind: ConfigKind::Common, file: "f.toml".into(), text: "greeting = false".into() });
assert!(!GREETING_ON.load(Ordering::Relaxed));
```

### `ConfigEvents::RELOADING`

A loaded config's file changed on disk; `config.text` is the new text. Not fired when the same text is saved again.

```rust
use pluto_lib::{ConfigEvents, ConfigKind, ModConfig};

ConfigEvents::RELOADING.register(|config| println!("{} changed", config.file));
ConfigEvents::RELOADING.invoke(&ModConfig { mod_id: "example".into(), kind: ConfigKind::Common, file: "f.toml".into(), text: "x = 2".into() });
```

### `ConfigEvents::UNLOADING`

A `Server` config was dropped as its server stopped; the next world reads it again. `Client` and `Common` configs stay until the game quits.

```rust
use pluto_lib::{ConfigEvents, ConfigKind, ModConfig};

ConfigEvents::UNLOADING.register(|config| assert_eq!(config.kind, ConfigKind::Server));
ConfigEvents::UNLOADING.invoke(&ModConfig { mod_id: "example".into(), kind: ConfigKind::Server, file: "f.toml".into(), text: String::new() });
```
