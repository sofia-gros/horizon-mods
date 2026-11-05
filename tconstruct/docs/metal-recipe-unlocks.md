# Metal storage and conversion recipe-book unlocks

Fifty-four existing recipes now have their original recipe-book unlocks:

- Block-from-ingots, ingot-from-block, ingot-from-nuggets and
  nugget-from-ingot for Amethyst Bronze, Cinderslime, Cobalt, Hepatizon,
  Knightmetal, Knightslime, Manyullyn, Pig Iron, Queen's Slime, Rose Gold,
  Slimesteel and Steel: 48 recipes.
- Raw Cobalt block compression and decompression: two recipes.
- Netherite ingot/nugget and Netherite Scrap/Debris Nugget conversion:
  four recipes.

Each source unlock requires the exact corresponding input item, OR an
already-unlocked recipe. These are direct item predicates, even where the
crafting recipe combines a fixed center item with Forge-tagged outer items.
No tag criterion is approximated with a fixed list. The native recipe reward
is specific to that recipe, with no display or achievement toast.

All source advancements are copied from Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, under
`src/generated/resources/data/tconstruct/advancements/recipes/misc/common/materials/`.
For the twelve metals, filenames are
`<metal>_{block_from_ingots,ingot_from_block,ingot_from_nuggets,nugget_from_ingot}.json`.
The remaining filenames are `raw_cobalt_block_from_raws`,
`raw_cobalt_from_raw_block`, `netherite_ingot_from_nuggets`,
`netherite_nugget_from_ingot`, `debris_nugget_from_ingot` and
`netherite_scrap_from_nuggets`, each with `.json` suffix.

Only recipe IDs and the host recipe-unlocked predicate key are adapted:
`tconstruct:common/materials/<name>` becomes the existing native
`tconstruct:<name>`, and source `recipe` becomes host `recipes`. All source
criteria, OR requirements and telemetry flags remain unchanged. The exact
source bytes, SHA-256 hashes and attribution are in
`tests/fixtures/metal_recipe_unlocks/`.

Existing crafting shapes, input tags, counts, item registrations and alloying
are unchanged. These unlocks do not supply the first ingot or nugget and do
not establish full natural acquisition or current-world recipe-tag parity.
Copper's separate compatibility recipe route, ore/tool smelting and blasting,
and conditional external material recipes remain outside this change.

Regression coverage was added for all source definitions, native recipe
ingredients/shapes/results and package inclusion. Tests were not run, as
requested. Brief static review only; inventory events, recipe-book UI,
crafting, resource reload, save/reload and LAN remain unverified. No CI or
version checks were performed.
