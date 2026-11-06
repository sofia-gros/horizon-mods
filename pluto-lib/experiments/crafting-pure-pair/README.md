# Pure native recipe derivation, legacy-contract candidate

**Excluded source only.** Ordinary manifests and all old hooks remain unchanged.
The current integration input is 17f44dc36fc2df28547328aa8554286ae965b208;
Horizon is 41f822b659789780f58d32c63ddfb6887ea1b8bf and Mixins is 5a64af93.
Actual complete-plan SHA is recorded in inputs.json. Its accepted target bindings
are not acceptance, type-checking or activation of this new candidate.

## Separate authority

Matching Pass/Deny admits or rejects a candidate. It never returns an item.
`native-derivation.rs` is a separate native full-stack derivation capability. It
reuses the existing host-independent `products::Pair<T>` value type without
changing that staging API or its explicit no-automatic-crafting contract.
A `PureRecipe` registers one pair callback and its two consumer-owned selectors
atomically under one ID. This native adapter accepts actual ItemStack values;
semantic ItemFacts, serialization or opaque wire are not reconstruction tokens.

Assembly and remainders recompute independently, as the old TConstruct hooks do.
There is **no new same-invocation or once-only consumption guarantee**. The
provider contract requires deterministic pure value derivation, no live storage,
RNG, global/context observations, callbacks with external effects, or publication.
A Rust fn pointer alone cannot enforce those obligations. For this migration,
the actual Overslime plan plus its `amount` and `refill` callees are source-pinned
and inspected. No claim is made about arbitrary future provider implementations.

Registry callbacks run after its read lock is released. Selectors run in ID order;
zero claims continue native, one claim derives, and multiple claims reject before
any derivation or input cloning. Duplicate registration does not replace. Claimed
None means unmatched, not native fallback. A pair with a wrong remainder length
fails closed; the old plan always creates exactly grid.len() remainder entries.
There is no actor/world cache, TLS, second matching registry, or mutation method.

## Old semantics retained

The complete actual plan's `lib-source-overslime` and
`lib-source-overslime-crafting` module sources are byte-identical to main08afa.
The entire old pair body is preserved, including full native clones, custom_data
patching, negative-value clamps in the original amount helper, refill arithmetic,
and original-bottle versus glass-bottle selection. TConstruct owns all item IDs,
selectors, material/component names and arithmetic.

- Assembly keeps the old row-major **nonempty compressed** grid and calls the
  same plan once. It bypasses native ingredient matching on a claimed marker,
  exactly as the old override does. It returns only that pair's output.
- Remainders use the original grid and exact unnormalized recipe ID, call the
  same plan once, and return only its remainders. Failure keeps the old
  `vec![None; size * size]` behavior. Malformed arbitrary caller dimensions do not
  acquire a new validation/authority guarantee.
- The proposed sole assembly HEAD evaluates existing matching policies first,
  then calls `after_matching`. A denial invokes neither derivation selectors nor
  the callback. The old anvil and melter policies only handle Shaped recipes;
  Overslime only selects Shapeless. Moving those two denials ahead of Overslime
  therefore does not change their current disjoint policy result. Other future
  matching policies retain deny-wins priority over derivation.
- Full native stack clone behavior is preserved, including runtime-only fields
  and retained Wire. Updating custom_data may make a copied raw patch unusable;
  retained bytes are not an assertion of canonical save/transport eligibility.
  The new generic staging API's stricter opaque refusal is not silently replaced.

## Proposed composition, not applied

The eventual ordinary change needs one native derivation module under crafting,
a consumer module, registration from the proved recipe-constructor bootstrap,
the shown single matching HEAD, and one Pluto-owned remainder HEAD. Only then may
the two old Overslime hooks be removed. The two stored-value denials must likewise
be registered before their old hooks are removed. The matching and new remainder
composition guards must pin exact helper/module bytes and reject competing target
hooks. This group does not relax either existing exclusive guard.

Pair registration and matching registration are distinct authorities. The existing
bootstrap still needs a concrete combined success/failure policy before ordinary
adoption; a later registration failure must not be described as rolling back a
previous successful registry insertion. No recipe should be returned from the
constructors after failed initialization, and existing remote-input take/lifecycle
limits must remain explicit. This group intentionally does not change bootstrap.

Station placement is not solved by this provider. Its local menu transaction,
block-entity persistence, UI/notification order, remote submission and separate
LAN authoritative transaction require their own native handler contract. Passing
an admission callback is not permission to mutate inventory.

## Finite comparison source

`render.py` builds a seven-file fixture using actual official Input, Recipe,
Template and ingredient declarations, real Items/StackData/Tag dependencies, the
unchanged old helper bodies, and only exact source slices of amount/refill. Those
two slices exclude unrelated Overslime modules/tests; they are not stub Game or
an assertion that the entire old dependency graph was compiled.

Sixteen proposed Rust cases cover four bottle types, partial/full/active and
malformed inputs, overworked arithmetic, holes/offsets, native unknown/runtime
fields, root/nested Wire, unused bottle metadata, source/recipe selector fallback,
independent recomputation, veto-before-selector ordering, no-provider, duplicate
ID/conflict handling and unlocked snapshot behavior. ItemStack PartialEq ignores
some runtime values, so the comparison explicitly inspects Wire patch bytes and
bundle selection as well. The tests use local registries; they do not prove
startup registration, actual installed HEADs, Game/menu/actor execution, network
transport, or complete crafting consumption.

Current result: 12 pure source checks PASS. All Rust tests, metadata, preparer,
actual candidate plan/apply and type checks are UNRUN. No workflow/trigger exists.
