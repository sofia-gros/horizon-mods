# Decorative masonry stonecutting

Part of high-priority #379, stacked on the fourteen direct-item routes in #858. This completes the remaining six source masonry stonecutting recipes and their acquisition unlocks:

- Any current Scorched Blocks tag member except the output becomes Scorched Bricks, Polished Scorched Stone or Chiseled Scorched Bricks.
- Any current Seared Bricks tag member except the output, or Seared Stone, becomes Seared Bricks, Fancy Seared Bricks or Triangle Seared Bricks.
- Every route consumes one block and yields one. The source difference/union tree is preserved exactly inside an explicit native ingredient wrapper.

## Current-world ingredient resolution

A cancellable station ingredient parser hook handles only `tconstruct:stonecutting_ingredient`. Its bounded resolver supports source item/tag leaves, nonempty union arrays and `forge:difference`. It subtracts identities without changing the surviving tag order, and deduplicates unions. Namespaced IDs must be valid and items registered. Unsupported fields, operators, malformed children or incomplete owned data yield an empty ingredient, never a weaker input match. Missing/empty tags contribute no members; a separate direct Seared Stone union alternative remains usable.

Resolution uses the `Items` supplied to the native station-table loader. World load/reload already builds that registry from the world's final pack stack before station recipes. There is no static tag expansion, cross-world cache or second crafting transaction. Native menu selection, consumption and result handling remain responsible for the operation. This is an explicit station-parser extension, not global Forge ingredient support; ordinary native ingredients retain their existing path.

The six source unlocks keep inventory criteria OR recipe-unlocked criteria and exact native recipe rewards. Discovery intentionally accepts the source tag even when the owned block is itself the output: source discovery and actual cutting eligibility are distinct. The existing advancement bridge now resolves `tconstruct:seared_bricks` and `tconstruct:scorched_blocks` from the current source/world tag snapshot rather than stale global host tags. Its existing reload/index invalidation behavior is unchanged.

## Scope and validation

These six conversions were excluded by #858 only because their difference ingredients were unsupported; this follow-on supplies that support. It does not add unrelated Forge operators, component-sensitive ingredients, recipes outside this family, or modify crafting-grid conversions. Full host transformation/compilation, actual station selection/consumption, reload, save/reopen and LAN behavior remain unverified.

Exact source recipe and advancement files/hashes are in `tests/fixtures/decorative_stonecutting`, under the existing TConstruct MIT license. A focused Rust fixture includes the production helper and all six native recipes; Python cases check exact adaptation, tag wiring and packaging. Tests were added but not run. Only brief static review and whitespace checking were performed. Builds, CI inspection, version checks and merging remain deferred.
