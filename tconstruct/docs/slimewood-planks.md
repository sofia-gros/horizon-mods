# Slimewood plank recipes

The pinned TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` Greenheart, Skyroot, Bloodshroom and Enderbark plank recipes and their item tags are adapted to native recipe data. Each matching normal or stripped log or wood yields four corresponding planks. Tags remain additive to permit other packs to extend them.

Source comparisons, registration checks and resource-overlay tests pass. Actual host recipe loading, gameplay crafting, recipe-book presentation, save/reload, LAN and final packaged-file inclusion remain unverified.

## Log-to-wood recipes

For each slimewood, the pinned source's normal and stripped log-to-wood recipes are also registered. Four matching logs in a 2×2 square produce three matching wood blocks. Source and registration tests pass; actual host crafting, packaging, recipe-book presentation, save/reload and LAN remain unverified.

## Slabs and stairs

The four slimewood slab and stair recipes follow the pinned source. Three matching planks make six slabs; six matching planks make four stairs. Source and registration tests pass. Host recipe loading, actual crafting and placement, packaging, recipe-book presentation, save/reload and LAN remain unverified.

## Buttons and pressure plates

Each slimewood has the pinned-source recipes for buttons and pressure plates: one matching plank yields a button; two horizontally adjacent matching planks yield a pressure plate. Source, registration and collision checks pass. Runtime recipe loading, crafting and redstone behavior, packaging, recipe-book display, save/reload and LAN remain unverified.

## Doors and trapdoors

The four slimewood door and trapdoor recipes follow the pinned source. Six matching planks in a 2×3 pattern make three doors; six in a 3×2 pattern make two trapdoors. Source, registration and collision checks pass. Runtime recipe loading, crafting and placement, packaging, recipe-book presentation, save/reload and LAN remain unverified.

## Fence gates

The four slimewood fence-gate recipes follow the pinned source: four vanilla sticks and two matching planks make one gate. Source, registration and collision checks pass. Runtime recipe loading, crafting and placement, packaging, recipe-book presentation, save/reload and LAN remain unverified.

## Fences and wooden rods

The four slimewood fence recipes follow the pinned source: four matching planks and two items from `forge:rods/wooden` make three fences. The native Forge tag adds the Forge 47.4.0 default stick without replacing other entries. Source, registration, collision and package-content checks pass.

The host currently prevents two content mods from owning the Forge namespace, and its native tag loader does not implement Forge's `remove` field. Full cross-mod or datapack override parity is therefore not established. Actual host recipe loading, crafting, placement, tag reload, recipe-book presentation, save/reload and LAN remain unverified.
