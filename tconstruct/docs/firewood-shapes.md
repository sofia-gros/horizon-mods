# Firewood building recipes

The pinned TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` recipes for Blazewood and Nahuatl fences, slabs and stairs are registered in native recipe format. For each material, six blocks craft six fences, three blocks craft six slabs, and six blocks craft four stairs. The source patterns, categories and groups are preserved.

These are data declarations checked against the pinned source archive. Runtime recipe loading, crafting in game, recipe-book presentation, save/reload and LAN have not been verified for these additions.

## Recipe-book unlocks

All six existing recipes now have their pinned source recipe-book unlock
advancements. Obtaining Blazewood or Nahuatl unlocks that material's slab,
stairs and fence recipes. The source criterion is named `has_item` for slabs
and stairs, and `has_planks` for fences; both require the exact full-block
item. Each retains the OR `has_the_recipe` alternative, no display, and only
its own recipe reward.

Source advancement paths are
`src/generated/resources/data/tconstruct/advancements/recipes/building_blocks/common/firewood/{blazewood,nahuatl}_{slab,stairs}.json`
and
`src/generated/resources/data/tconstruct/advancements/recipes/decorations/common/firewood/{blazewood,nahuatl}_fence.json`.
Native adaptations flatten the recipe IDs and change the recipe-unlocked
predicate's `recipe` key to Horizon's `recipes` key. Exact fixtures, hashes
and attribution are in `tests/fixtures/firewood_recipe_unlocks/`.

No recipe inputs/counts, block behavior or base-block casting route changes.
This does not add fence gates or establish complete survival acquisition.
Regression coverage was added for source criteria, native recipe mapping and
package inclusion, but was not executed. Brief static review only; inventory
events, recipe-book UI, crafting, resource reload, save/reload and LAN remain
unverified. No tests, CI or version checks were run for the unlock change.
