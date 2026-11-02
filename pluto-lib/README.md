# PlutoLib

**Reusable Rust APIs for Horizon mods.**

Define items and blocks with a small Rust API, then make them available in the Creative inventory. PlutoLib builds on Horizon’s mod system; it is not a mod loader.

![The PlutoLib demo Creative tab in Horizon](docs/demo-tab.png)

*Items and blocks registered by the PlutoLib demo, running in Horizon.*

## What works today

PlutoLib generates template-based item and block declarations and prepares Creative inventory data. For newer Horizon versions, the [single native library](docs/native-installation.md) provides [Creative preparation](docs/native-creative.md) using Horizon's own page controls; the [paging adapter](docs/creative-overflow.md) targets an older host.

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

[Get started](docs/getting-started.md) · [API guide](docs/api.md) · [Demo source](examples/demo.rs) · [Native Creative preparation](docs/native-creative.md) · [Native installation](docs/native-installation.md) · [Experimental item behaviors](docs/item-behaviors.md) · [Component transport prototype](docs/component-transport.md) · [Tool rules](docs/tool-rules.md) · [Combined native demo](experiments/unified-target/README.md)

## Compatibility

The unreleased `0.1.0` development version has been tested on Linux with pinned Horizon and Mixins commits. See [compatibility](docs/compatibility.md) for the verified combination, the initial demo's Mixins limitation, and development targets.

The existing Python Creative helper remains available; see its [guide](docs/creative-tabs.md).
