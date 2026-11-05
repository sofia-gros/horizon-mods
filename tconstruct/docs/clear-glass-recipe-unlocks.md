# Clear Glass recipe-book unlocks

The 49 existing Clear Glass crafting recipes now have their source recipe-book
unlock advancements:

- Clear Glass unlocks clear panes and all sixteen dyed Clear Glass block recipes.
- Each dyed Clear Glass block unlocks its matching pane recipe.
- Clear Glass Pane unlocks all sixteen direct pane-dyeing recipes.

The source inventory criterion OR recipe-unlocked criterion is preserved.
Rewards and the host's `recipes` predicate refer to existing native recipe IDs.
No dye possession is added to the advancement criteria: the source unlocks
these recipes from the relevant glass item. Crafting still requires its existing
ingredients and quantities, including the native Forge dye tags where used.

These are recipe-book records, not new visible goals, casting recipes, material
sources or changes to transparency/connected rendering. Existing Forge namespace,
tag merging and tag-removal limitations are unchanged.

Unmodified source fixtures and hashes from Tinkers' Construct
[`f4e77d43`](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2)
are in `tests/fixtures/clear_glass_recipe_unlocks`. Focused source, recipe mapping
and package regressions are included but were not run in this implementation
pass. Actual recipe-book display, inventory events, crafting, save/reload and
LAN remain unverified. This is part of #379's high-priority recipe-unlock work.
