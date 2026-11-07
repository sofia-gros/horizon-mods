# Read-only crafting-match admission

`pluto_inventory_api::crafting::matching` is a shared deny-only boundary before
native recipe matching and assembly. It does not place ingredients, consume
items, grant knowledge, replace an output, identify a recipe persistently, or
interpret any consumer's component keys. TConstruct's material schema, marker,
recipe eligibility and crafting formulas stay in TConstruct. This slice does
not activate a production consumer.

## Contract and costs

Register an ID, cheap-interest function and read-only callback during startup.
One process-lifetime registry snapshots registrations in lexical ID order.
Empty or duplicate IDs are rejected without replacing a registration. Interest
functions and callbacks execute outside the registry lock; registration during
a callback affects only a subsequent snapshot. Every interested callback runs,
even after a prior denial. Any denial wins; the first denying ID is reported.
`Pass` means native continuation, never authorization or a replacement result.

With no providers the adapter returns before constructing recipe facts. With
providers it builds borrowed ingredient views and cheap fixed-template facts;
these may allocate small vectors. It captures input/result components only if
at least one interest function selects the candidate. There is no throughput
or allocation benchmark claim. Register before the first relevant recipe
query; no automatic invalidation or replay of previously computed results is
provided by registration.

Recipe facts reuse the existing ingredient view and preserve accepted IDs,
order, custom JSON, nested ingredient shape, cell holes and shaped geometry.
Input facts preserve the native cropped row-major cells, holes, width, height
and left/top offsets. They are not a compressed stack list or a reconstructed
full crafting grid. Fixed output facts come from native `Template::create` and
are a read-only observation, not a proposed output. Special recipe ingredients
and dynamically assembled results are explicitly unavailable, not empty.

Item facts use the existing shared component view. Retained wire data, including
nested retained wire, makes component information unavailable; missing facts
must not be treated as an empty component map. This is semantic read-only state,
not complete private StackData/Wire identity or write authority. A consumer may
fail closed when the facts needed for its rule are unavailable.

The returned evaluation is only an ephemeral callback snapshot. The native
adapter evaluates it synchronously with the same recipe facts. Consumers must
not retain actor/world state, infer persistent recipe ownership, mutate slots,
or perform delayed matching from a callback. No recipe ID is invented: the
canonical native function receives a recipe value, not its registration ID.

## Native paths and composition

At official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, both
`Recipes::find_with` and `Recipes::recipe_matches` call
`Recipes::assemble_if_matches`. Pluto owns a cancellable HEAD there:

- Denied returns native `None` before shaped/shapeless matching or special
  assembly. In a find loop the next candidate can still be considered.
- Native continuation executes the original matching and result construction
  unchanged. The veto cannot turn a native mismatch into a match.
- This is separate from the automatic-placement veto and explicit ghost preview.
  Passing manual matching never overrides automatic-placement denial.

The complete-plan guard conservatively reserves this target for Pluto. An
additional early cancellation could bypass admission, and a return rewrite
could restore a denied output. Every competing phase is rejected until an
alternative composition is proven. The exact official function fingerprint,
pristine file hash, helper bytes, target, bindings, phase and priority are bound
by the guard. Consumers must remove their old raw matching hooks when adopting
the registry. Installing both is not a supported migration.

This boundary does not control direct inventory writes or custom matching paths
that bypass the native function. The fixture does not establish live gameplay,
LAN delivery, consumption atomicity, or special-recipe behavior. Full packaged
Mixins apply and host type checks are tracked separately from source/fixture
proof in [the validation report](crafting-match-veto-validation-20261105.md).

## Bounded consumer comparison

The test-only callback under `tconstruct/tests/rust/crafting-match-policy`
compares visible material/shape/result rules with the exact old source gate and
planner from MOD commit `8c56bb52b6b77fae6115e894af9e369a5c3f9e08`.
All TConstruct IDs and schema remain in that test directory, not the shared API.
The comparison covers source-generated fake-metal forms, not every possible
private stack state or the entire old PR. Unknown retained-wire components are
explicitly denied rather than claiming private-identity equivalence. Production
startup registration, removal of the old hook and full consumer validation
remain later migration work.
