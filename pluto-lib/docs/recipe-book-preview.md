# Explicit recipe previews

This is a bounded part of the shared recipe-book feature group. PlutoLib owns
native presentation access. Consumers keep recipe selection, component matching,
material rules and all decisions about whether a recipe may be placed.

## Public capability

`pluto_inventory_api::recipe_book::preview::Preview` exposes `select` and `show`
with host-independent book kinds and read-only entry facts. The gameplay layer
uses the existing `horizon_client::pluto_inventory` re-export, not another API
instance. PlutoLib's private native adapter lends the capability synchronously
through `game::recipe_book::pluto_preview::with_game`.

The associated selection type privately retains the complete native entry.
Before showing it, the adapter checks the current local book, entry position,
book category, nonnegative display ID and grid fit again. It compares all exposed entry
fields, recursive ingredient structure, ordered alternatives and the shared
observable stack snapshot, including components, retained wire and runtime
bundle selection. A mismatch rejects the request before native ghost presentation. No recipe is selected implicitly.

This is observable current-entry equality, not full private native identity or a
persistent world/session token. The shared snapshot helper cannot inspect the
private Wire decoded baseline: equal stale wire-patch bytes can conceal distinct
private baselines. Selection freshness therefore is not proven for that hidden
state. This read-only preview contract must not become write authority.
Reloading an identical entry can satisfy it. It must not be used to establish
index ownership, authorize inventory changes or defer a transaction. The
capability adds no cache, registry, hook, world state or automatic activation.

`show` delegates to native ghost presentation. It does not clear/move slots,
send a placement request, grant knowledge or cancel another caller's placement
code. **Component-sensitive preview-only consumers must retain their placement
vetoes.** A shown ghost is neither proof of craftability nor permission for an
ID-only placement path. Remote clients are rejected by this local capability;
no guest/server routing or fallback is added.

## Explicit ingredient overlays

The generic `ingredient_previews` helper overlays alternatives by ingredient
index. Missing overrides preserve the provided fallback; explicit empty ones
remain empty. Duplicate or out-of-range indexes fail without changing input.
There is no policy inference from the alternatives.

The thin native `recipe_book::pluto_preview_cells::cells` helper returns cloned
full ItemStacks at native 3-by-3 overlay positions, using native ingredient
ordering and fallback lookup. It preserves component-rich existing entry
previews, rejects oversized layouts and invalid maps, and omits empty cells.
It does not change `Book::placements`, placement eligibility or ghost logic.
Furnace overlays use the center cell. The public operation is read-only; it
installs no overlay hook.

## Persistent RecipeIndex ownership remains unresolved

The desired follow-up would atomically replace explicitly owned definitions,
compare exact prior records, reject collisions and let each consumer choose its
allocation policy. It must not infer ownership from an ID prefix or a pointer.
The existing fake-material and Travelers families use different allocation
strategies; a shared API must preserve that choice rather than normalize it.

An actual latest-official Mixins feasibility plan for state on
`horizon_client_lib::recipe_book::RecipeIndex`, initialized in
`RecipeIndex::load`, failed with `E_UNSUPPORTED_CONTROL_FLOW`: closures, async,
try/?, unsafe and const blocks are unsupported injection contexts. The native
loader contains closures. The probe installed nothing and no ledger is shipped.
Static inspection found the sole explicit RecipeIndex construction there;
derived Default/Clone does not prove state initialization. StationData is
all-public and does not satisfy the state target's private-field rule. Its
WorldTables owner also has complex initialization. These last two alternatives
were source-inspected, not separately planned.

An external ledger plus record comparison cannot distinguish a reload producing
identical records. Therefore this slice makes no persistent ownership claim,
does not copy the native loader body, and does not change display-ID allocation
or any consumer's placement path. See the adjacent validation report for the
actual evidence, completed native integration gate and remaining lifecycle gap.

## Consumer migration map (not activated)

The old fake-material family at MOD
`8c56bb52b6b77fae6115e894af9e369a5c3f9e08` has three distinct boundaries:

- Its preview HEAD hook cancels placement only after family-prefix and nonempty
  component-preview checks. Those checks and the decision to cancel remain in
  TConstruct; only native selection/presentation should use the capability.
- `Recipes::placement` and decorated `Book::placements` independently refuse
  ID-only material selection. Neither guard may be removed merely because the
  shared preview succeeds. Hand-arranged crafting checks are also unchanged.
- Its old overlay skips missing overrides and takes the first duplicate. The
  shared overlay preserves native fallback and rejects duplicates. Validated
  fake-material decoration produces one override per ingredient, where the
  results agree. Any migration of malformed or partial maps must explicitly
  choose a fallback rather than claim drop-in equivalence.

Travelers at MOD `a8fc1e18e9c819c93a60cd50aea87dc8a099efaa` uses ordinary
native ingredients and leaves normal placement enabled. It must not inherit the
fake-material family's preview-only restriction. Its gap-reusing display-ID
allocation and the fake-material family's append-after-others allocation stay
outside this preview service. These source observations are a migration plan,
not proof that either original consumer has been ported or tested here.
