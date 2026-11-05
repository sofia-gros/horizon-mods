# Native Turtle Scute acquisition compatibility

The pinned TConstruct 1.20.1 material recipe accepts `minecraft:scute`.
Horizon 26.3 uses `minecraft:turtle_scute`; its vanilla item renamer records
that exact rename, and growing turtles dispatch the vanilla `turtle_grow`
loot table. Without adaptation, the registered native item does not match
TConstruct's craftable Turtle material ingredient in the Part Builder.

Both manual resource preparation and bundled startup now adapt the generated
original-source layer, only when the native item exists and the old ID does
not. Exactly two item fields change:

- `tools/materials/turtle_scute`: ingredient item becomes the native ID.
- `tables/recycling/turtle_helmet`: the five-scute output becomes the native ID.

Material identity, costs, counts, pattern, tool and recipe conditions stay as
pinned. Archives and the player's resource inputs remain unchanged. No item
alias, save migration, or world datapack rewrite is introduced; normal later
world pack overrides still take precedence. The owned startup cache fingerprint
changes so previously prepared caches are rebuilt with the compatibility fix.
Legacy registries containing `minecraft:scute` are left alone. Existing manual
resource outputs must be regenerated with `--tconstruct-prepare` into a fresh
output directory; startup deliberately does not rewrite player-owned prepared
resources. Worlds and saved inventory need no migration.

Tests cover pinned recipe/material parity, both preparation hooks, cache
invalidation, registry gating, exact-field changes, repeated preparation and
persisted generated JSON. A host transaction test checks material lookup,
source cost, recycling output parsing and output serialization round-trip.
This does not claim an in-game turtle breeding, Part Builder click, save/reload
or multiplayer session was run. Turtle Shell modifier and severing support
remain outside this bounded acquisition change.
