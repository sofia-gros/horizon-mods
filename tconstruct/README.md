# Tinkers' Construct for Horizon

A port of [Tinkers' Construct](https://github.com/SlimeKnights/TinkersConstruct) to Horizon: build tools from parts, melt and cast metals, repair and modify your equipment.
Not released yet (see [the status](../README.md#status-under-recovery)).

## What has been run in a real game

On Windows with Horizon `ebcfa226` and the PlutoLib features that currently pass their guards (`tools/smoke-local`):

- The game starts and loads the mod's content.
- In survival, the crafting grid turns flint into a pick head, two sticks into a handle or a binding, and those three parts into a flint pickaxe (data recipes in `content/data/tconstruct/recipe/`).
- The Part Builder block, placed and right-clicked, opens its screen.

## The library and the tools made on it (new)

The original's library, ported as plain Rust, lives in `mixins/tconstruct/api/library` (`cargo test --manifest-path mixins/tconstruct/api/library/Cargo.toml`; tests in seconds, and they read the original's generated data when `.local/tconstruct-source` is there). It is built into the game with the API crate and loads the original's data: 83 materials, 329 modifiers and 45 tool definitions, with no problems. Tools made on it carry their materials, stats, modifiers and free slots in the stack's `minecraft:custom_data`.

These commands run in a real game (`tools/smoke-local`, the full probe checks each):

| Command | What it does |
|---|---|
| `/tconstruct preview <tool> <material>` | says the stats, modifiers and slots a tool of one material would have |
| `/tconstruct build <tool> <materials...>` | gives a tool, one material for each part in order (the last covers the rest) |
| `/tconstruct part <part> <material>` | gives a tool part that carries its material |
| `/tconstruct craft` | makes the first tool whose parts are in the inventory, from the materials they carry |
| `/tconstruct upgrade <modifier>` | adds a level of a modifier to the first tool with the slots for it (slot cost, tools and the items it takes from the original's recipes; the items are taken from your inventory) |

A tool made this way acts in play: it digs with its mining speed where its definition says it is effective and its harvest tier can harvest the block, it wears against its own max damage, it hits for its attack damage, and its tooltip describes it. Not yet: haste and fatigue, the game's cooldown scaling on hits, ranged tools, armor, the Tinker Station and Part Builder screens on PlutoLib (the legacy ones still work on their own data), the item costs of the multi-level and swappable modifier recipes, removing modifiers, and most of the modifier modules (6 of 165 carry behaviour; the rest are kept as data). The roadmap ([docs/roadmap.md](../docs/roadmap.md)) has the plan and the decision log.

## What exists but has not been run in a game

About 290 injections into Horizon (287 accepted by the pinned Horizon Mixins, all of them compile) and their helper code cover: the Part Builder, Tinker Station and anvils; the Melter, Smeltery,
Foundry and casting; slime blocks, trees and islands; materials and traits; the guide book. They stage and compile, but nothing beyond the two checks above was exercised: no crafting through the Part Builder, no mining, no
save/reload, no multiplayer. The legacy pickaxe is a template item; the tools of the library above are material-driven.

Parked because the pinned Mixins refuses the hook: mining speed (`source-host-mining-context`, `source-client-mining-context`) and the Tinkers Chest pick color (`tinkers-chest-pick-color`).
The official launcher drops a whole mod when Mixins refuses one hook, so the packaged `tconstruct.hnmod` is built from the slice in `tools/smoke-local/slice.json`. It also leaves out the 17 injections that edit Horizon files which Mixins breaks when it rewrites them (slime islands, slime and ender trees, saplings) until Mixins is fixed.
**Verified:** the official launcher builds this package together with the old PlutoLib package (27 minutes the first time), and `tools/smoke-local/smoke.py` passes on the game it produced.

## Direction

These injections are the first port, written before PlutoLib. They are frozen by the lint baseline and are replaced, feature group by feature group, by calls to [PlutoLib](../pluto-lib/README.md); see [docs/roadmap.md](../docs/roadmap.md). The open pull requests are classified in [docs/migration/pr-classification.md](../docs/migration/pr-classification.md).

## Layout

`content/` data and recipes · `assets/` resources · `mixins/tconstruct/` the bundle (`horizon-mixin.toml`, helpers) · `reference/` data read at run time · `tools/` resource preparation, packaging, staging · `tests/` Python tests · `third_party/` notices.
Build and check: [../docs/development.md](../docs/development.md).
