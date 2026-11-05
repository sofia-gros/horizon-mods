# Current-world tagged acquisition unlocks

This change extends the bounded advancement-tag bridge from PR #675 and adds
fourteen source recipe-book unlocks for already existing native recipes:

- Pattern, Copper Can, Gold Bars, Seared and Scorched Ducts, and Seared Fluid
  Cannon.
- Greenheart, Skyroot, Bloodshroom and Enderbark planks.
- Earth, Sky, Ichor and Ender Congealed Slime.

Source inventory predicates use eleven tags: `forge:rods/wooden`,
`forge:ingots/{copper,gold}`, `tconstruct:{greenheart,skyroot,bloodshroom,enderbark}_logs`,
and `forge:slimeball/{earth,sky,ichor,ender}`. Legacy `tag` predicates become
the host's native `items: "#tag"` form, not fixed lists of current members.
Both the inventory candidate index and final predicate use the same resolved
current-world snapshot for these tags and the existing
`tconstruct:modifiable/multipart` tag. Workstation discovery additionally
uses the source `tconstruct:seared_blocks` tag, as described in
[workstation discovery unlocks](workstation-discovery-unlocks.md).
Unrelated tags retain host behavior.

Missing, invalid or empty snapshots match nothing for the managed tags,
including when the native resource table still has stale members. The
existing snapshot refresh invalidates the inventory candidate cache. A
tag-only reload does not synthesize an inventory event or revoke an earned
advancement; an already-held newly eligible item is evaluated on the next
inventory change. This does not claim exact source reload timing or full
advancement-engine compatibility.

All fourteen retain original criterion names and OR recipe-unlocked
alternatives. Rewards point to existing native recipe IDs. Recipe-unlocked
`recipe` becomes host `recipes`. There is no display or achievement toast.
Existing crafting recipes, ingredients/counts and machine behavior do not
change. The Duct filter behavior and Fluid Cannon operation are separate
from discovering their crafting recipes.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, under
`src/generated/resources/data/tconstruct/advancements/recipes/`:

- `misc/tables/pattern.json` and `misc/smeltery/copper_can.json`.
- `decorations/common/gold_bars.json`.
- `redstone/smeltery/{seared,scorched}/duct.json` and
  `redstone/smeltery/seared/fluid_cannon.json`.
- `building_blocks/world/wood/{greenheart,skyroot,bloodshroom,enderbark}/planks.json`.
- `building_blocks/common/slime/{earth,sky,ichor,ender}/congealed.json`.

Exact source bytes, SHA-256 hashes, source paths and native mapping are in
`tests/fixtures/tagged_recipe_unlocks/manifest.json`. The fixture README
retains attribution. This does not add the colored slime storage-block
unlock recipes from the separate registration PR or all other missing
tag-dependent advancements.

Added regression coverage exercises the extended bridge with replaced,
missing, invalid and empty membership, cache refresh, stale native members,
unrelated tags and non-item registries, plus source criteria, recipe mapping
and package inclusion. No tests were executed, as requested. Brief static
review only: host compilation, injection application, inventory/recipe
events, recipe-book UI, crafting, resource reload, save/reload and LAN remain
unverified. No CI or version checks were performed.
