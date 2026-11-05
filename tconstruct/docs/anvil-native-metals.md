# Pig Iron and Knightmetal Anvil crafting

The pinned source `tconstruct:anvil_metal` tag includes both Pig Iron and
Knightmetal storage blocks, which were already registered in the native port
but missing from its fixed Anvil recipe variants. Both now have Tinkers' Anvil
and Scorched Anvil crafting routes: three matching metal blocks across the top,
one matching-category support block in the center and three across the bottom.
The support categories remain the existing native Seared/Scorched block tags.

The four new recipes retain metal identity in `tconstruct:anvil` custom data
and select new material-specific item models. They reuse the existing Anvil
placement, rendering, drops and workstation code. The existing mixed-ordinary-
alloy fallbacks also accept both blocks and still produce the fallback variant.
No compatibility-material guard or fake storage block behavior changes.

Knightmetal uses its existing storage sprite. Pig Iron uses the existing
`block/storage/pig_iron/top` sprite, also its storage block's particle texture,
for the native Anvil's single material texture. This is a bounded appearance
adaptation: the source block's six face textures are not reproduced on the
Anvil. No external artwork or guessed texture path is added.

The recipe generator includes both metals. Each existing Anvil discovery
advancement now covers eighteen recipes: eleven ordinary metals, six
compatibility variants and one mixed fallback. The source inventory criterion
still uses the current world's Anvil Metal tag, and all recipe events remain OR
alternatives. Discovering a recipe does not supply its ingredients.

Source evidence is already retained in
`content/reference/tconstruct-tags/data/tconstruct/tags/items/anvil_metal.json`
and the corresponding `forge` storage-block tag files, plus the original
Anvil recipes in `content/reference/tconstruct-recipes.zip`, pinned to
Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`.

This does not implement arbitrary datapack metal variants, exact current-world
crafting tag parity or first-metal acquisition. Existing native fixed inputs
and single-texture rendering boundaries remain. Generated content and source
mapping received a brief static review. Regression coverage was added but not
run; no tests, compilation, CI inspection, version checks or merge occurred.
Actual crafting, item/world appearance, placement, drops, save/reload, recipe
book, guest synchronization and LAN remain unverified for these new variants.
