# Chest and Modifier Worktable discovery

The existing Tinkers' Chest recipe now unlocks on Pattern possession, using
the pinned source criterion and OR recipe-unlocked alternative. This is a
recipe-book advancement, separate from workstation operation or item storage.

The source Modifier Worktable recipe is one retextured recipe. The native
port already represents its supported rock choices as thirteen fixed crafting
recipes: Andesite, Basalt, Blackstone, Calcite, Cobbled Deepslate, Cobblestone,
Deepslate, Diorite, Dripstone Block, Granite, Mossy Cobblestone, Stone and Tuff.
One hidden advancement now rewards all thirteen. It preserves source
discovery from the current world's `tconstruct:seared_blocks` item tag, OR
the unlocked event for any of those native recipes. Each event has a separate
string-valued `recipes` condition; no unsupported array predicate is used.

This extends PR #760's bounded snapshot bridge by one named tag. The inventory
candidate index and final item predicate both read current-world membership.
Missing, invalid or empty snapshots match nothing, with no stale native tag
fallback. Existing snapshot refresh invalidates the index, without generating
an inventory event or revoking earned progress. Tag-only changes are evaluated
on the next inventory change. Other tags and non-item registries retain the
previous behavior.

Source advancements are from Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, at
`src/generated/resources/data/tconstruct/advancements/recipes/decorations/tables/{tinkers_chest,modifier_worktable}.json`.
Exact source bytes, SHA-256 hashes and attribution are in
`tests/fixtures/workstation_discovery_unlocks/`. Native adaptations are the
recipe-ID mappings, `recipe` to `recipes`, legacy `tag` to native `items:
"#tag"`, and mapping one Worktable recipe to its thirteen supported native
variants. No display or achievement toast is added.

Existing recipe ingredients, output metadata, counts and workstation behavior
are unchanged. These unlocks do not implement arbitrary retextured recipes,
custom rock choices, full current-world ingredient parity or first-material
acquisition. The alternate log/table Crafting Station recipes remain outside
this change. Anvil recipe discovery is covered separately in
[Anvil discovery](anvil-discovery-unlocks.md).

Regression coverage was added for source criteria, complete supported-variant
reward mapping, native recipe outputs and package inclusion. The existing
world-tag fixture now includes the Seared Blocks tag. No tests or compilation
were run, as requested. Brief static review only; injection application,
inventory/recipe events, recipe-book UI, crafting, reload, persistence and LAN
remain unverified. No CI or version checks were performed.
