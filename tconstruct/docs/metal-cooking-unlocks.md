# Metal cooking recipe-book unlocks

Six source recipe-book advancements now unlock seven existing native cooking
recipes: Raw Cobalt and Cobalt Ore blasting, Cobalt Shard blasting, and Steel
and Knightmetal shard smelting/blasting.

Source advancements are from Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, beneath
`src/generated/resources/data/tconstruct/advancements/recipes/misc/common/materials/`:
`cobalt_ingot_blasting`, `cobalt_nugget_blasting`,
`steel_nugget_{smelting,blasting}` and
`knightmetal_nugget_{smelting,blasting}`, each with `.json` suffix.

The source Cobalt ingot recipe accepts Raw Cobalt or Cobalt Ore, but its
inventory unlock specifically requires Raw Cobalt. The native port already
splits that recipe into `cobalt_raw_blasting` and `cobalt_ore_blasting`.
One hidden advancement retains the Raw Cobalt inventory criterion and rewards
both native recipes. Its OR alternatives accept the unlocked event for either
native recipe, reflecting the original single recipe's two inputs. It does
not invent an ore-possession criterion. Each recipe event uses a separate
string-valued `recipes` condition supported by the host; no array predicate
is introduced.

Cobalt Shard's source recipe maps to existing `cobalt_shard_blasting`.
Steel and Knightmetal retain their native recipe names. Those five unlocks
preserve their exact source shard-possession criterion and OR
recipe-unlocked alternative. Knightmetal cooking accepts the existing raw
nugget tag, while its source unlock specifically requires the TConstruct
Knightmetal Shard. No broader tag-based discovery is added.

All recipe IDs are adapted to existing native IDs, and the source
recipe-unlocked predicate's `recipe` key becomes the host's `recipes` key.
Exact source fixtures, hashes and attribution are retained in
`tests/fixtures/metal_cooking_unlocks/`.

Cooking inputs, output counts, times and experience are unchanged. No geode
loot, ore generation, furnace behavior or alloy acquisition is added.
Recipe-tag compatibility and full natural acquisition remain separate work.

Regression coverage was added for exact source/adapted criteria, split
Cobalt rewards, existing cooking recipes and package inclusion. Tests were
not run, as requested. Brief static review only; inventory events, recipe
events, recipe-book UI, cooking, resource reload, save/reload and LAN remain
unverified. No CI or version checks were performed.
