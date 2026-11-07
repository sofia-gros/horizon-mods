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

## Where the work is going

[docs/roadmap.md](roadmap.md) is the plan: finish PlutoLib's modules in Fabric API order, move Tinkers' direct injections onto them (the lint baseline shrinks to zero), then re-absorb the open pull requests per [docs/migration/pr-classification.md](migration/pr-classification.md).
