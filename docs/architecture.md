# Architecture

## Products

| Directory | Product | Version and tag |
|---|---|---|
| `pluto-lib/` | **PlutoLib**, Horizon's Fabric API (see [design principles](../pluto-lib/docs/design-principles.md)) | `pluto-lib-vX.Y.Z` |
| `tconstruct/` | **Tinkers' Construct** for Horizon | `tconstruct-vX.Y.Z` |

Both live in this repository, each with its own version and release. Mods depend on PlutoLib; PlutoLib depends only on Horizon.

## How a mod meets Horizon

```text
Horizon (official)  <-- injections --  mod bundle (horizon-mixin.toml, Rust helpers, an [api] package)
      ^                                         |
      |  native hook points (stable, additive)  |  [dependencies] pluto_lib = "^0.1"
      +------------------------------------ PlutoLib ([api] package, Fabric-like events and registries)
```

- A **bundle** is a folder with `horizon-mixin.toml`: injections into Horizon functions, helper Rust modules, state, and optionally an `[api]` package (a trait mod, entry point `hooks()`).
- **Horizon Mixins** applies the bundles to Horizon's source. We never edit Horizon or Mixins.
- **Native hook points** are Horizon's own stable, only-ever-added functions (`native_*`). Hooking only these lets a mod name the hook API version it needs instead of pinning a commit.
- **PlutoLib** (`pluto-lib/mod`) is a trait mod. Mods declare it under `[dependencies]` in their `horizon-mixin.toml` and call its `[api]` crate; the official launcher links everything. Verified on the pinned Horizon with an experimental pair of mods.
- `pluto-lib/mod/api/src/pure/` is host-independent (`base`: events with phases, `Identifier`, `ActionResult`, the support matrix of stubs; and the Fabric-like modules built on it) and tested alone; `hooks.rs` is the only file that names Horizon types and turns Horizon's hook points into those events.

## Layout

```text
pluto-lib/
  mod/                 the new PlutoLib mod (horizon-mixin.toml, api/ crate)
  mixins/              the old unified PlutoLib bundle (pluto_native_creative): migrated into mod/ module by module
  tools/               packaging and the composition guards
  tests/ guards/ docs/ design documents, tests, guard evidence
tconstruct/
  content/ assets/     data and resources of the mod
  mixins/tconstruct/   the legacy bundle: ~290 direct injections and helper sources (to be replaced by PlutoLib calls)
  reference/           data read at run time (recipes, hashes)
  tools/ tests/        resource preparation, packaging, staging; Python tests
tools/                 repository tools: lint, line endings, slice, probe, release packaging and validation, smoke-local/
docs/                  roadmap, architecture, development, releasing, migration/ (PR classification), branding/, images/
.github/workflows/     CI (mixins.yml), per-product release workflows, weekly upstream drift
```

## World generation boundaries

`BlockEditor::place_feature` now passes through PlutoLib's concrete live-level adapter into the active native configured-feature catalog. Native code still owns the level RNG copy/write-back, updates and placement result. Four real bone-meal fungus cases check this boundary; they do not prove custom feature registration or natural generation.

The general feature-type/codec adapter is checked by an independent game consumer: decode configuration before generation, pass a borrowed editor/origin, and delegate only unregistered types to native placement. This is not an Ender-tree-specific hook. Subsequent root/decorator extensions must retain active configuration, original holder/provider identities, ordered root matching, native parent fallback, waterlogging and the supplied feature RNG. Root/decorator callbacks must run inside the feature's placement context, never by scanning completed chunks or retaining borrowed state in TLS. The Java root-selection adapter has pure tests and five live root fixtures. Decorator codecs now have an explicit-call registry and the Java leaf-vine loop has an editor adapter; an independent live fixture checks its attachment/length/obstruction and failure behaviour, with images inspected. The explicit native-tree adapter passes a live oak fixture with extra vines and finalization; the separate root-override registry does not establish Ender-tree completion.

The pinned Mixins accepts root decoding but still rejects generic root placement. Decoding a custom type into an approximate native tree would lose those semantics. In particular, a native rule-based provider may continue after a matching provider returns no state; Java extra-root selection chooses the first matching provider before sampling. Such a rewrite needs equivalence evidence, not just a successful decode. Live placement and generation-worker registration must therefore be reviewed separately; a successful live adapter is not permission to replace natural-generation timing with chunk-load callbacks.

PlutoLib owns child modules in the native tree/live-catalog/level modules, without injecting generic methods. An isolated feasibility build first proved access to native root simulation/waterlogging/tracking; the subsequent live oak fixture checks the concrete adapter and extra-decorator finalization. Per-position custom root writes now pass live tests; complete root simulator replacement remains unsupported.

Native leaf-vine decoration also cannot simply stand in for the custom Java decorator: it chooses the fixed `minecraft:vine` default and consumes side draws in W/E/N/S order, whereas the ported custom loop uses N/E/S/W and the configured vine. An adapter must preserve the supplied RNG, ordered leaf list, default-state attachment properties, tracked writes and the final native leaf-distance/edge-shape passes. The live native feature path clones and writes back the level RNG; callbacks must use that same placement stream, not independently draw from the level while the clone is active.

The native-tree adapter moves the internal owned-state/editor contract into a PlutoLib module in `horizon-world`; the client bridge reexports the same types, avoiding a reverse world-to-client dependency. Pure bridge tests compile that actual shared source. A tree orchestration entry reuses native roots/trunk/foliage and decorators, then accepts extra writes before the existing distance/edge-shape passes. The full native build and live oak fixture pass for `BlockEditor::place_tree`: native logs/leaves plus extra vines, final leaf distances and an edge-shape callback beside a tracked extra marker. Callback errors preserve prior writes and skip finalization; invalid tree configuration never reaches the callback. Extra writes are tracked before finalization, and the advanced copied RNG is written back on success or unwind. Nested features/trees inside extra callbacks conservatively fail without writing until nested tracking exists. Appending decorators is not equivalent to arbitrary interleaving with native decorators.

Per-position root overrides retain the base native root simulator, clipping and trunk offset. A separate host binding pass compiles every ordered holder/provider descriptor before writes (256 each, native provider node budget). The Java adapter selects the first matching holder, samples that provider alone on the supplied RNG, returns its state for native waterlogging/tracking, or requests native parent placement only on no match. Invalid bindings and sample/callback errors stop, never try another variant or parent. The adapter requires `get_optional_state` to return a state: native `get_state` would otherwise silently reuse the current block when an optional provider produces nothing. The root-aware adapter passes its native build and five real bone-meal fixtures: weighted first-match selection, waterlogging, native muddy-root parent fallback, selected-provider no-state failure and invalid unselected bindings rejected before callbacks/writes. All five images were inspected. Root-type decoding in ordinary native JSON and complete Ender Tree/data-schema integration remain unfinished.

## Where the work is going

[docs/roadmap.md](roadmap.md) is the plan: finish PlutoLib's modules in Fabric API order, move Tinkers' direct injections onto them (the lint baseline shrinks to zero), then re-absorb the open pull requests per [docs/migration/pr-classification.md](migration/pr-classification.md).
