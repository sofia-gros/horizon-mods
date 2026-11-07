# Seared and Scorched stonecutting

Part of high-priority #379. Adds fourteen source stonecutter routes for already registered masonry blocks, separately from the existing crafting-grid recipes:

- Seared Bricks and Seared Cobble: slabs, stairs and walls (six routes).
- Seared Stone and Seared Paver: slabs and stairs (four routes).
- Scorched Bricks and Scorched Road: slabs and stairs (four routes).

One input block yields two slabs, one stair or one wall. Native recipe IDs use the existing flat content namespace and end in `_stonecutter`, so crafting recipe IDs, patterns, counts and discovery are unchanged.

Each route includes its source inventory acquisition criterion OR recipe-unlocked criterion, rewarding the same native stonecutter recipe ID. Source conditions require the actual base block; owning the output or a brick item does not replace that requirement. No material costs are added or reduced.

## Adaptation and limits

The source's direct-item ingredient object becomes the host's item-ID string. Its `result` string plus top-level `count` become the host's `{id, count}` result object. The advancement maps the source recipe ID to the native ID and uses the host's `recipes` event predicate key. All other source advancement fields remain unchanged.

This slice intentionally excludes six decorative block-conversion recipes that use Forge difference ingredients: three Scorched block conversions and Seared Bricks/Fancy Bricks/Triangle Bricks conversions. Substituting a static tag membership list or allowing a recipe to consume its own output would not preserve those source semantics. Travelers synthetic recipe discovery and broader world-local recipe-book support remain separate work.

No stonecutter menu, recipe-index, crafting transaction, block behavior, world generation or source tag changes are made here. The recipes use the existing ordinary native stonecutting pipeline; live acquisition, selection, consumption, shift-taking, reload and LAN behavior remain unverified.

Exact source recipe/advancement files and hashes are in `tests/fixtures/masonry_stonecutting`, under the existing TConstruct MIT attribution. Focused tests compare all fourteen source conversions, counts, criteria, registration and content packaging. Tests were added but not run. Only brief static review and whitespace checking were performed; builds, CI inspection, version checks and merging were deferred.
