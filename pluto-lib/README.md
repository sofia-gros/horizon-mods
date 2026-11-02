# PlutoLib

**Reusable Rust APIs for Horizon mods.**

Define items and blocks with a small Rust API, then make them available in the Creative inventory. PlutoLib builds on Horizon’s mod system; it is not a mod loader.

![The PlutoLib demo Creative tab in Horizon](docs/demo-tab.png)

*Items and blocks registered by the PlutoLib demo, running in Horizon.*

## What works today

PlutoLib generates template-based item and block declarations and prepares Creative inventory data. Creative tabs currently use available positions on Horizon's existing screen; switching between overflow tabs is planned.

## Start with a small example

```rust
use pluto_lib::Registry;
use std::path::Path;

let mut registry = Registry::new("my_mod")?;
registry.item("gem", "minecraft:amethyst_shard")?;
registry.block("gem_block", "minecraft:amethyst_block")?;
registry.write_to(Path::new("generated-content"))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

This writes content declarations. Include them with your mod’s manifest and assets, as shown in the demo.

[Get started](docs/getting-started.md) · [API guide](docs/api.md) · [Demo source](examples/demo.rs)

## Compatibility

Verified demo: Horizon `b6c936f82b08798057161bdfa577e3aac53fb91e` on Linux.

Planned native integration target: Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`; not exercised by this demo. Compatibility with other revisions is not promised; source changes may require rebuilding. PlutoLib is not a Fabric compatibility layer.

The existing Python Creative helper remains available; see its [guide](docs/creative-tabs.md).
