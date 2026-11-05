# Travelers material crafting

The five original `crafting_shaped_materials` recipes for Travelers goggles,
chestplate, pants, boots and shield now project supported material combinations
into the current world's ordinary native shaped-recipe table. Each result is
assembled by the same material-stat, trait, upgrade and final-normalization
pipeline used by the Tinker Station. Inputs are consumed by the native menu,
not by a second custom inventory transaction.

The projection uses the world's resolved material recipes, item tags, material
predicates, definitions and registered items. It requires one material identity
for all occurrences of each of the two parts, and preserves original pattern
sizes and per-cell costs. Items worth a nugget or block cannot replace an exact
one-unit ingredient. Ordinary ingredient tags are expanded using the current
snapshot. Projection is rebuilt after initial source capture and reload, with
no global cross-world material cache. Unsupported input, missing assembly
context, ambiguous identity scans or excessive expansion remove the affected
projection instead of providing a guessed result.

The native menu already uses world recipes for preview, take, shift-click and
remainders, including the server's LAN crafting-table menus and TConstruct
Crafting Station. Two fixed-host reads still used global resource recipes:
local crafting preview and Crafter dispense. Narrow injections now route those
reads through the world table too. Existing reload generation guards reject a
stale LAN table click after its result changes. These are code paths connected
by this change, not claims of a live client/LAN test.

## Bounded support

- Ordinary material-recipe inputs are supported. Component-sensitive fake
  ingots, reused tool parts and goat-horn instruments are not projected.
- Mixed variants of the same base material, such as different wool colors,
  remain unsupported. The source can discard the variant in that situation;
  this slice requires an identical full material identity.
- Material predicates or normal ingredient cells that overlap are refused.
  The source's more general inventory-wide ordering is not approximated.
- Native `recipe/` world overrides of the five contributed paths take precedence
  over the legacy source recipe. Conditional native-format overrides disable
  the projection; legacy source conditions use the existing snapshot resolver.
- Synthetic IDs are reserved under `tconstruct:travelers_native/`. Recipe-book
  discovery, original recipe-ID advancement rewards, and arbitrary conflicting
  datapack ordering are not completed by this slice.
- At most 4,096 combinations per source recipe and 8,192 total are projected.
- This establishes a bounded ordinary acquisition route, not completion of all
  armor/shield effects or all material crafting recipe types.

## Verification

Seven focused production-parser tests passed. They check five equipment shapes
and counts, identity-separated alternatives, exact material values, malformed
patterns, fake/component-sensitive inputs, overlapping predicates and refreshed
candidate sets. Formatting and patch whitespace were checked. Full Mixins
transformation, host type-check, native-menu transaction tests, save/reopen and
live LAN/Crafter validation are deferred to the final high-priority batch.

Specification: Tinkers' Construct 1.20.1
[`f4e77d43`](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2),
MIT, particularly
[`ShapedMaterialsRecipe`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/recipe/material/ShapedMaterialsRecipe.java),
[`MaterialValueIngredient`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/recipe/ingredient/MaterialValueIngredient.java),
and the five exact recipes already retained in `tconstruct-recipes.zip`.
