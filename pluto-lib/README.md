# PlutoLib

PlutoLib helps Rust developers prepare content for Horizon mods. It does not load mods; Horizon loads the generated content. Its first Rust API creates item and block declarations and adds Creative tabs to prepared inventory data.

```rust
use pluto_lib::Registry;

let mut registry = Registry::new("my_mod")?;
registry.item("gem", "minecraft:amethyst_shard")?;
registry.block("gem_block", "minecraft:amethyst_block")?;
registry.write_to(std::path::Path::new("generated-content"))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

This writes Horizon content declarations under `generated-content/data/my_mod/horizon/`. Add a manifest and assets before loading the content. Blocks also receive an item by default. For a complete example, see the [guide and demo](docs/getting-started.md); see the [API notes](docs/api.md) for validation and limits.

## Compatibility

The target combination for this unreleased API is Horizon commit `b6c936f82b08798057161bdfa577e3aac53fb91e` and horizon-mixins commit `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. Content declarations target Horizon's content API 0.2. This combination has not yet been validated with the demo in a running game. Compatibility with other revisions is not promised; source changes may require rebuilding. PlutoLib is not a Fabric compatibility layer.

## Current scope

The Rust API generates template-based item and block declarations and transforms a prepared Creative inventory. It does not yet register runtime behavior or dynamically resize the Creative screen. Block entities, equipment behavior, GUI, books, inventory, recipes, events, persistence and sync are planned, not implemented. The existing Python Creative helper and its [guide](docs/creative-tabs.md) remain available to existing users.
