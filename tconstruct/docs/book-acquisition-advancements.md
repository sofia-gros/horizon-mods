# Guide acquisition advancements

The two introductory guide acquisitions use the original TConstruct 1.20.1
advancements from revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`.
Obtaining Materials and You satisfies `tconstruct:tools/materials_and_you`;
obtaining Puny Smelting satisfies its child `tconstruct:smeltery/puny_smelting`.
Both use the source `minecraft:inventory_changed` criterion, so any legitimate
acquisition counts. The item icon field is adapted to Horizon's current `id`
format. The original advancement background is covered by
`third_party/tconstruct/LICENSE`.

An isolated Linux resource fixture on Horizon `02c8607ee8ebd216f2f535c1eb7b62b572830df2`
and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a` confirmed the local
starting book and crafted Puny book produced the two saved advancement records.
A save/reload preserved both criteria and their original completion timestamps.
A LAN guest given the Puny book received its own two records; the host did not
receive the guest's Puny advancement. The guest's advancement screen showed the
two nodes. The test used supplied materials/items and does not establish a
survival acquisition route. The original background asset is included, but its
exact display parity has not been established. The generated content archive
was inspected and contains both definitions and the background; packaged-game
installation, final-main integration and Windows are not yet verified. Puny's
item and recipe require the Puny guide integration.

These are two advancement paths, not the complete original progression. Other
source advancements, custom triggers, conditions and guide content remain
unfinished or unverified.

## Fantastic Foundry entry

`foundry/fantastic_foundry` now records acquiring Fantastic Foundry, under the
existing Materials and You root. The source criterion accepts the book from
any acquisition method, not only crafting. Its display retains the source task
frame, toast, translations and disabled chat announcement, with `item` adapted
to the host's `id` icon field. It has no reward.

The separate `recipes/misc/fantastic_foundry` advancement unlocks the existing
book-and-Nether-Grout recipe when Nether Grout enters inventory, or records the
recipe if it is already unlocked through the recipe-unlocked event. These are
OR alternatives, not two required steps. The source recipe ID
`tconstruct:common/fantastic_foundry` is mapped to the existing native ID
`tconstruct:fantastic_foundry`; the source `recipe` predicate key is adapted to
the host's `recipes` field. No duplicate recipe or new crafting requirement is
introduced. Both definitions derive from TConstruct `f4e77d43`.

Tests cover the source conditions, parent, display, native recipe mapping,
item declarations, translations and inclusion in the packaged content. Actual
inventory/recipe events, recipe-book UI, progress persistence and LAN actor
attribution remain unverified in a running world. This is the obtainable entry
to Foundry progression, not its completion: Scorched Alloyer is not registered,
the Foundry structure requires an unsupported custom container-open trigger,
and the Blazing Blood goal needs tank-content predicate compatibility. Those
nodes, and the existing Plate Armor branch's missing ancestors, remain open.
