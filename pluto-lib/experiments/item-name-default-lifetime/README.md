# Default item-name lifetime: source-only contract

Official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`; official Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`. This group is separate from frozen item-name-removal codec32. There is no manifest, installed API, native execution or new CI workflow here. The four Rust cases are prepared source for a later actual items/world fixture, not a new independently invented library.

## Concrete gap and lifetime evidence

- `crates/horizon-world/src/item_defaults.rs`: `shared(root)` returns an immutable Arc from a root-keyed Weak cache while any load owns it. After all strong references disappear, the next call reads files/hooks again. `read(root)` is an explicit fresh read; it is not automatically the snapshot used by Items.
- `crates/horizon-items/src/items.rs` 725–750: both `Items::load` and `Items::catalog` enter `load_with`, which calls `shared(root)`. Lines 890–907 retain the component key list, but keep values in `custom_components` only for non-`minecraft:` keys. The item-name payload is not stored there. The returned Items has no native Defaults Arc field.
- `item_data.rs` 259–266 creates an ItemStack with `extra: None`. `item_name` 360–363 reads only an explicit preserved patch component. Neither operation owns a resource-root/default snapshot.
- `item_tooltip.rs` 1101–1112 resolves custom_name, then explicit item_name, then a String fallback. It does not receive Items. Game `stations.rs` 2132–2161 has its own custom/book/item_name/ominous/potion/head/standard fallback ordering. `titles.rs` 65–83 uses another fallback through `name_line` for the held-name HUD. Replacing one path is not proof of all display paths.

Holding the **native Defaults Arc for the lifetime of Items is unsafe for reload semantics**: a second load of the same root can reuse the old snapshot while that Arc is alive. A companion that simply retains this Arc does not solve the lifetime problem.

## One minimal connection proposal, not an enabled implementation

During an existing native Items load, borrow/hold the same native snapshot only across that load. Copy the selected raw item-name payloads into an immutable **Items-owned** field, then release the native Arc before publishing the Items. Do not retain Text already resolved in one locale: preserve translation key, arguments and style. The copied payload's lifetime ends with that Items snapshot; a later Items load gets its own projection. No global map, pointer identity, root-key cache, TLS or standard key hidden in `custom_components`.

A read-only resolver would receive the real Items snapshot and stack. It must preserve the native custom-name and written-book behavior, never let a prototype name override an explicit `!minecraft:item_name` removal, use explicit per-stack names before prototype lookup, and leave all existing special/standard fallbacks intact when unmanaged or absent. A removal suppresses the item-name component/default, not an independent custom_name or the existing standard fallback. It must not mutate ItemStack, count, Wire, save encoding, protocol or registry identity.

This requires proof of **two distinct boundaries**:

1. Items-owned storage and population for every supported constructor/load path. Mixins `docs/effects-and-state.md` explicitly says state initializers are standalone functions with no constructor-local capture. `raw` cannot simply be referenced from an initializer. The accepted identity `Items::load` call probe does not prove Items state insertion, constructor coverage, population, `catalog`, reload or failure cleanup. A state constructor plan/check is still needed. Unsupported creation paths must retain native behavior, not receive an invented default snapshot.
2. Actual display sites with Items context and a Text result. The existing `styled_name` String fallback cannot preserve a styled default merely by returning a string. Both station tooltips and held-name HUD require reviewed composition boundaries; no copied Game body, implicit global context or new TLS is proposed. The exact guest/removal candidate must also be composition-checked before relying on remote tombstones.

The private owner/population machinery belongs to Pluto's internal native layer. An author-facing convenience wrapper may remain thin over that same owner/lookup contract; it must not create a second cache or grant item mutation authority. No public wrapper is added before these native boundaries are proved.

## Four prepared native cases

`catalog_tests.rs` imports actual `horizon_items`, `horizon_world` and `serde_json`; it uses the native `PackOverlay::NONE`, shared/read loaders, Items::catalog, ItemStack and encode_components. It requires the exact official path dependencies/lock in a later bounded fixture. It is not wired into codec32 or automatically executed.

1. The default key survives catalog loading, its vanilla value does not become custom metadata, and a newly created stack has no materialized name.
2. Holding the native Arc keeps the prior snapshot across a disk change; releasing it permits a new catalog while an old catalog keeps its already copied custom metadata.
3. A catalog does not itself retain the raw native Defaults Arc.
4. Native name reads do not materialize the prototype or alter the explicit patch encoding. Custom-name behavior remains separate from a removal marker.

These cases deliberately document existing behavior, not a completed default-name connection or whole default-component lifecycle. No native hook is installed by the test. Actual mod-content Item generation, constructor/state support, Game rendering, resource reload scheduling and network delivery remain unproved. The source guard only checks test names, input identity and separation from frozen codec32; it is not Rust execution.
