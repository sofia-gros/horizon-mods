# Basalt furnace and Cheese recipe-book unlocks

Three existing crafting recipes now have their original recipe-book unlocks:
the Smooth Basalt alternative for a Blast Furnace, Cheese Block from four
Cheese Ingots, and four Cheese Ingots from one block.

The Blast Furnace alternative unlocks on obtaining Smooth Basalt, preserving
the source criterion name `has_smooth_stone` despite that name. Cheese block
compression unlocks on a Cheese Ingot, and decompression on a Cheese Block.
Both use the source criterion name `has_cheese`. Each retains an OR
recipe-unlocked alternative and rewards its own existing native recipe.
No achievement display or toast is added.

Source advancement paths are from Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, beneath
`src/generated/resources/data/tconstruct/advancements/recipes/`:

- `building_blocks/common/basalt_blast_furnace.json`
- `food/common/cheese_block_from_ingot.json`
- `food/common/cheese_ingot_from_block.json`

Only source recipe paths and the host predicate key are adapted:
`tconstruct:common/<name>` becomes the existing native `tconstruct:<name>`,
and recipe-unlocked `recipe` becomes `recipes`. Exact source fixtures,
hashes and attribution are in `tests/fixtures/common_recipe_unlocks/`.

Existing ingredients, counts, cooking behavior, Cheese food behavior and
first Cheese casting are unchanged. No additional food or utility block is
registered, and Gold Bars' tag-dependent unlock remains outside this change.
This does not establish full natural acquisition.

Regression coverage was added for source criteria, existing recipe inputs,
shapes and counts, and package inclusion. Tests were not run, as requested.
Brief static review only; inventory/recipe events, recipe-book UI, crafting,
resource reload, save/reload and LAN remain unverified. No CI or version
checks were performed.
