# Give an item a behavior

The experimental item API lets a native mod register a handler for a namespaced item ID. The independent [wand demo](../examples/wand/mixins/pluto_wand/behavior.rs) increments its own charge on use and displays that value in the tooltip:

```rust
use pluto_item_api::{ItemBehavior, ItemData, UseOutcome};

struct Counter;
impl ItemBehavior for Counter {
    fn on_use(&self, data: &mut ItemData) -> UseOutcome {
        data.set("example:uses", data.get("example:uses").unwrap_or(0) + 1);
        UseOutcome::Handled
    }
}
pluto_item_api::register("example:wand", Counter)?;
```

Register each ID once in the shared native `pluto-item-api` crate before its hooks run. The demo's Mixins bundle calls its registration function before use, remote use and tooltip hooks. The library exposes the same contracts as `pluto_lib::item` for Rust consumers; native handlers must use the shared staged `pluto-item-api` instance. The API currently stores named signed 64-bit integers. It does not define a mana system or a magic effect.

`Pass` runs standard Horizon behavior and discards the handler's staged data changes. `Handled` consumes the use and commits them. `Cancel` consumes the use without committing. Duplicate registration is rejected. Tooltips receive read-only data. The adapter stores fields under `plutolib:data` in the stack's `minecraft:custom_data`, preserving other fields; unknown data types in that reserved compound cause the use to fail rather than being overwritten. Treat the reserved compound as owned by PlutoLib.

The handler is intended to run in the host's item-use path, including a LAN guest's action as processed by the host. The remote-side hook identifies a registered item as usable; it does not run the handler or predict a data change. Custom-data network adapters derive from the Tinkers port; see [their notice](../mixins/pluto_items/NOTICE.md). This path, serialization, save/reload and guest synchronization **have not yet been compiled or tested in game**. In particular, coexistence with another native mod intercepting the same custom-data network hooks is unverified. Do not rely on this prototype for authoritative multiplayer data yet.

## Prepare the demo

From `pluto-lib/`, generate a new content directory with a prepared Horizon resource file:

```sh
mkdir -p .work
cargo run --locked --example wand_demo -- .work/wand-demo /path/to/horizon-resources/creative-inventory.json
```

The demo uses Minecraft's existing blaze-rod texture and creates its own item declaration. The native build needs both `mixins/pluto_items` and `examples/wand/mixins/pluto_wand` staged as sibling bundles, using the pinned Horizon and Mixins commits in [compatibility](compatibility.md). The generated Creative inventory must be used in a separate resource overlay, as in [getting started](getting-started.md). Formal build, launch instructions, a game capture and platform claims will be added after validation.
