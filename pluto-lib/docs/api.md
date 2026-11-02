# Registration API

`Registry::new(namespace)` accepts a lowercase content namespace other than `minecraft`. `item(name, template)` and `block(name, template)` declare names under it using vanilla template IDs. Names can contain slash-separated resource path segments. A block also creates a block item, so item and block names may not collide. `write_to` requires a new output directory and creates Horizon's content declaration files. It does not write the manifest, models, textures or translations.

To override an item's template stack limit, call `item_with_max_stack(name, template, limit)` with a value from 1 through 99. For example, `registry.item_with_max_stack("gem", "minecraft:amethyst_shard", 16)?` writes:

```json
{"template": "minecraft:amethyst_shard", "max_stack_size": 16}
```

`item(name, template)` leaves the limit unspecified, so Horizon uses the template's default. Invalid limits fail before registration and do not reserve the name. This is a preparation-time content declaration; it is separate from the native `pluto_api::item` behavior API. Horizon's official `native_items` hook accepts the same setting as `horizon_mods::content::Item { max_stack_size: Some(16), .. }`; `Registry` does not itself register native items. The separate demo-only native hook that sets a limit of 200 does not change the declaration limit.

For block declarations, `block_with_properties` overrides selected template properties:

```rust
use pluto_lib::{BlockProperties, Registry};

let mut registry = Registry::new("example")?;
registry.block_with_properties("gem_block", "minecraft:amethyst_block", BlockProperties {
    hardness: Some(2.0),
    explosion_resistance: Some(6.0),
    requires_tool: Some(true),
})?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Each field is optional; use `..Default::default()` to inherit the rest. Hardness must be finite and nonnegative, or -1 for an unbreakable block; explosion resistance must be finite and nonnegative. Invalid properties are rejected without reserving the name. `block` continues to inherit all properties. These are content declarations, not runtime block behavior.

To also specify emitted light and a sound template, use `block_with_settings`:

```rust
use pluto_lib::{BlockPresentation, BlockProperties, Registry};
let mut registry = Registry::new("example")?;
registry.block_with_settings("lamp", "minecraft:amethyst_block", BlockProperties::default(),
    BlockPresentation { light: Some(14), sounds: Some("minecraft:anvil") })?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Light is 0–15; an explicit 0 emits no light, while `None` inherits the template. `sounds` names a vanilla **block** whose sound settings are copied, not a sound event. Registry validates the ID format; Horizon checks whether the block exists during resource preparation. Both settings may be omitted independently.

`add_creative_tabs(&inventory, &tabs)` returns a copy of the JSON document. A tab has a namespaced ID, nonempty title and icon, `TOP` or `BOTTOM` row, column 0–6, and an ordered list of SNBT stacks. Duplicate IDs and occupied visible positions are rejected; Horizon's hotbar position may be reused, matching the existing Python helper. The API does not validate SNBT syntax or whether its item IDs are registered; supply valid stacks for registered items. The original inventory is never changed.

This is preparation-time registration, not Fabric's runtime registry or a runtime tab event. Template registration does not attach runtime behavior; the experimental [Tool rules adapter](tool-rules.md) is separate. Vanilla templates and available properties are determined by the target Horizon version. The demo's Creative overlay must be installed separately from its content mod.

For more tabs than the screen has positions, see the experimental [Creative paging API](creative-overflow.md). It requires the bundled Mixins adapter and uses the pinned Horizon item parser to populate search; unlike `add_creative_tabs`, it checks the supplied stack syntax.
