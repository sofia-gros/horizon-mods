# PlutoLib

**Reusable Rust APIs for Horizon mods.**

PlutoLib owns reusable APIs and their official Horizon/Mixins adapters; consuming
mods own their content and gameplay policy. See the
[shared API boundary](docs/shared-api-boundary.md) for the dependency contract
and the current migration limits.

Define items and blocks with a small Rust API, then make them available in the Creative inventory. Block declarations can override properties such as emitted light and sound templates. PlutoLib builds on Horizon’s mod system; it is not a mod loader.

![The PlutoLib demo Creative tab in Horizon](docs/demo-tab.png)

*Items and blocks registered by the PlutoLib demo, running in Horizon.*

## What works today

PlutoLib generates template-based item and block declarations and prepares Creative inventory data. For newer Horizon versions, the [single native library](docs/native-installation.md) provides [Creative preparation](docs/native-creative.md) using Horizon's own page controls; the [paging adapter](docs/creative-overflow.md) targets an older host.

## Start with a small example

```rust
use pluto_lib::Registry;
use std::path::Path;

let mut registry = Registry::new("my_mod")?;
registry.item_with_max_stack("gem", "minecraft:amethyst_shard", 16)?;
registry.block("gem_block", "minecraft:amethyst_block")?;
registry.write_to(Path::new("generated-content"))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

This writes content declarations for a gem that stacks to 16 and a block. Include them with your mod’s manifest and assets, as shown in the demo.

[Get started](docs/getting-started.md) · [API guide](docs/api.md) · [Demo source](examples/demo.rs) · [Native Creative preparation](docs/native-creative.md) · [Native installation](docs/native-installation.md) · [Experimental item behaviors](docs/item-behaviors.md) · [Component transport prototype](docs/component-transport.md) · [Tool rules](docs/tool-rules.md) · [Inventory planning](docs/inventory.md) · [Combined native demo](experiments/unified-target/README.md)

## Compatibility

The unreleased `0.1.0` development version has been tested on Linux with pinned Horizon and Mixins commits. See [compatibility](docs/compatibility.md) for the verified combination, the initial demo's Mixins limitation, and development targets.

The existing Python Creative helper remains available; see its [guide](docs/creative-tabs.md).
# Native fluid reads

The native creative bundle includes a read-only `pluto_fluid::first` helper for a block entity's first host-registered fluid tank on a given face. It reads the same `fluids` state and face mapping as the native host; it does not mutate fluid and does not resolve external capability systems or fluid tags. This candidate requires integration and host validation.
