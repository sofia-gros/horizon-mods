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
- Synthetic IDs are reserved under `tconstruct:travelers_native/`. They now have
  world-local recipe-book index entries, but automatic discovery, original
  recipe-ID advancement rewards, and arbitrary conflicting datapack ordering
  are not completed by this slice.
- At most 4,096 combinations per source recipe and 8,192 total are projected.
- This establishes a bounded ordinary acquisition route, not completion of all
  armor/shield effects or all material crafting recipe types.

## Recipe-book indexing follow-up

After world projection, each generated recipe receives one learnable native
RecipeIndex display, preserving source category, group and show-notification
metadata. All five pinned recipes use the equipment category. Native Book
construction keeps the assembled result's materials, stats and upgrades.

Only the reserved Travelers index and recipe metadata entries are replaced.
Other recipe families retain their display numbers. Available display gaps are
reused in deterministic recipe-ID order so repeated rebuilds do not drift upward
beside another synthetic family. Missing/invalid projection clears stale entries.
The old book, known-display cache and stale ghost/click state are invalidated.

The normal native craftability and placement paths stay enabled for these bounded
ordinary item ingredients. No alternate inventory transaction or material-blind
fake-ingot placement is introduced. Actual UI/placement behavior is unverified.

This is indexing support, not automatic discovery. Native recipe commands can
learn the indexed variants after projection. The five source acquisition goals
and dynamic rewards remain separate. Early native saved-book validation may still
discard synthetic recipe knowledge before projection on reopen; Travelers has no
saved-book handoff in this change. Remote clients do not install authoritative
projections, and LAN display/content synchronization remains unimplemented.

The added planner metadata and index allocation unit cases plus source/wiring
regressions were authored but **not run**. Brief static review and whitespace
checking only; no compilation, native injection checks, CI/version checks or merge.
The earlier parser test report below predates this follow-up and does not validate it.

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
