# Tinkers' Construct for Horizon

A port of [Tinkers' Construct](https://github.com/SlimeKnights/TinkersConstruct) to Horizon: build tools from parts, melt and cast metals, repair and modify your equipment.
Not released yet (see [the status](../README.md#status-under-recovery)).

## What has been run in a real game

On Windows with Horizon `ebcfa226` and the PlutoLib features that currently pass their guards (`tools/smoke-local`):

- The game starts and loads the mod's content.
- In survival, the crafting grid turns flint into a pick head, two sticks into a handle or a binding, and those three parts into a flint pickaxe (data recipes in `content/data/tconstruct/recipe/`).
- The Part Builder block, placed and right-clicked, opens its screen.

## What exists but has not been run in a game

About 290 injections into Horizon (287 accepted by the pinned Horizon Mixins, all of them compile) and their helper code cover: the Part Builder, Tinker Station and anvils; the Melter, Smeltery,
Foundry and casting; slime blocks, trees and islands; materials and traits; the guide book. They stage and compile, but nothing beyond the two checks above was exercised: no crafting through the Part Builder, no mining, no
save/reload, no multiplayer. The pickaxe is a template item, not yet a material-driven tool.

Parked because the pinned Mixins refuses the hook: mining speed (`source-host-mining-context`, `source-client-mining-context`) and the Tinkers Chest pick color (`tinkers-chest-pick-color`).
The official launcher drops a whole mod when Mixins refuses one hook, so the packaged `tconstruct.hnmod` is built from the slice in `tools/smoke-local/slice.json`. It also leaves out the 17 injections that edit Horizon files which Mixins breaks when it rewrites them (slime islands, slime and ender trees, saplings) until Mixins is fixed.
**Verified:** the official launcher builds this package together with the old PlutoLib package (27 minutes the first time), and `tools/smoke-local/smoke.py` passes on the game it produced.

## Direction

These injections are the first port, written before PlutoLib. They are frozen by the lint baseline and are replaced, feature group by feature group, by calls to [PlutoLib](../pluto-lib/README.md); see [docs/roadmap.md](../docs/roadmap.md). The open pull requests are classified in [docs/migration/pr-classification.md](../docs/migration/pr-classification.md).

## Layout

`content/` data and recipes · `assets/` resources · `mixins/tconstruct/` the bundle (`horizon-mixin.toml`, helpers) · `reference/` data read at run time · `tools/` resource preparation, packaging, staging · `tests/` Python tests · `third_party/` notices.
Build and check: [../docs/development.md](../docs/development.md).
