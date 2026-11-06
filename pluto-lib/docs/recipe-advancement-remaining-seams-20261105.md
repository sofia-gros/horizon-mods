# Remaining recipe and advancement seams, 2026-11-05

This is source inspection, not consumer activation or gameplay validation.
Official Horizon is `624c4f6265c02296fbe8a80f029dd0d987fb94c0`; no fork is
used. Consumer references are MOD #941 at
`8c56bb52b6b77fae6115e894af9e369a5c3f9e08` and #943 at
`a8fc1e18e9c819c93a60cd50aea87dc8a099efaa`. Existing shared contracts and their
proof cutoffs remain authoritative; a source mapping does not validate an old
PR head retroactively.

## Retire rather than extract: world recipe selection

#943's `travelers-crafting-preview.rs` replaces `Game::craft_recipe` after the
native result, solely to select `game.recipes().find_with` with map scale.
Official `game.rs:9491–9498` already performs that selection. Its current
`Game::recipes` (`game.rs:20126–20131`) selects world tables, then synchronized
recipes or resource fallback. The old correction no longer supplies a missing
shared facility.

#943's `travelers-crafter-result.rs` replaces a historical
`self.res.recipes().find` expression with `game.recipes().find`. Official
`game/machines/crafter.rs:95–129` already selects world recipes with
`find_with` and the level's saved-map context; remainders use the same world
recipe table and original grid. Renaming the old selector or wrapping the new
result with `find` would discard that context. Remove these redundant correction
hooks during the consumer port, with focused regression evidence, instead of
creating a Pluto service merely to retain them. No removal is performed here.

## Covered generic admission, pending consumer policies

#941 has three read-only material/content gates at
`Recipes::assemble_if_matches`: `fake-material-recipe.rs`,
`anvil-material-recipe.rs`, and `survival-melter-recipe.rs`. The shared
crafting-match admission service supplies borrowed ingredient structure,
input geometry and read-only input/fixed-result component facts. The first has
a bounded test-only comparison; the latter two have only been inspected for API
fit. Their material selection, shape rules and empty-tank eligibility stay in
TConstruct. Unknown component availability must not be treated as empty data.
All old hooks must be removed when callbacks are activated; the complete-plan
guard intentionally rejects installing both.

## Distinct remaining result/remainder authority

`source-overslime-craft-assemble.rs` does more than deny: it derives a new tool
stack from an input and returns a replacement output. Its paired
`source-overslime-craft-remainders.rs` preserves or converts individual bottle
slots. The deny-only matching API cannot express either operation, and `Pass`
must not be broadened into a grant or output replacement.

A later coherent service would need explicit native-stack derivation and
validation, result/remainder agreement, original-grid indexing and refusal
semantics. It must not use a process-global cache or recipe pointer as a promise
that separately invoked output and remainder calls are one transaction. The
current semantic component view is not complete private StackData/Wire write
authority. This remains unimplemented; no consumer policy has moved.

## Candidate: station ingredient parsing

#941's `source-stonecutting-ingredient.rs` owns a HEAD at `stations::ingredient`.
Official `stations.rs:497–499` delegates to `Ingredient::parse`, with an empty
ingredient on failure. The helper is used for smithing fields as well as
stonecutting (`stations.rs:557–576`), so a shared contract must not mislabel it
as stonecutting-only or assume a caller's recipe type.

The reusable part is raw-value dispatch and read-only access to the supplied
item registry and ordered tag members, followed by a native Ingredient adapter.
The TConstruct marker, source JSON grammar, Forge difference interpretation,
ID rules, recursion bounds and eligibility remain consumer policy. A proposed
shared callback needs explicit Unsupported versus handled-empty results,
deterministic conflict handling, no-provider native fallback, and exact
ordered-result validation. No parser API or new native hook is installed by
this inspection document.

## Already extracted advancement mechanisms

Metadata opt-in/scopes, predicate callbacks, explicit event submission,
per-world tag/index state and loaded-tree projection are shared. Their
TConstruct criteria, tag allowlist/snapshot production, dynamic goal IDs and
startup activation remain consumer work. No additional generic advancement
mechanism was established by this pass; remaining direct consumer files do not
by themselves justify a second registry or duplicate host hook.
