# Proposed official contract, not an applied extension

## 1. Problem and minimum semantic addition

The official host separately computes an output and later recomputes remaining
items. A mod can return two different pairs across those calls. Deterministic
registration, identical input values, or a recipe-ID cache does not fix this.
`on_take` is too late: the cursor, inventory or bundle may already own output.

The minimum addition is **one paired preparation seam plus a private native
operation that consumes that exact pair**. A preparation hook alone is not
sufficient. The official host must carry the chosen pair to every supported
authority's allocation/consumption boundary, before any result leaves storage.

The proposed public seam returns only owned output/remainder values. It does
not expose slots, publication, recipe ownership, arbitrary consumption counts,
or a mod-callable commit token. One occupied input cell still loses one item,
then its supplied remainder is processed by the native route's rules.

The host's operation is short-lived and bound to the actual invocation and
storage owner. It does **not** require persistent RecipeIndex ownership, a
pointer-keyed cache, or storing a receipt between frames. This is a proposal
for an official implementation, not a claim that current Mixins can add it.

## 2. Proposed preparation decisions

Names below describe a contract, not a compile-ready signature or accepted API.
The host visits the current recipe table in existing registration order. For
each candidate, after authority checks and applicable deny-only admission:

- `Unsupported`: this provider does not own the candidate. Continue the exact
  native candidate path. No provider means no component snapshot or new cache.
- `NoMatch`: the owned candidate does not match. Continue to the next candidate,
  as native `None` does. Do not run native assembly for that owned candidate.
- `MatchedEmpty`: stop selection without a result, as native `Some(None)` does.
- `Prepared(output, remainders)`: select this existing candidate and retain the
  entire pair. It is a proposal, not permission to publish anything.
- `Refused(reason)`: invalid, unavailable, conflicting or unsupported authority
  for an owned candidate. Abort the attempt; do not fall back to a placeholder
  native result or quietly try a less constrained recipe.

Only one owner may claim a candidate. Conflict is refusal, not first-wins or
last-wins. An interest query must be read-only; it is not a second preparation.
The selected provider is invoked at most once for a candidate in one attempt,
and its selected pair is never regenerated during commit. A retry is a new
attempt, not replay of the previous receipt. No global purity assumption is
needed to keep each pair internally consistent.

Candidate context contains current native recipe identity/definition and table
generation, original square grid, crop offsets, holes/order and read-only item
facts needed by the policy. It must not invent IDs or compress nonempty cells.
The native host retains actual full stacks; semantic component DTOs are not
reconstructed into live items or used as authority/equality tokens.

This seam is separate from matching/placement vetoes. A deny remains a deny;
`Prepared` cannot override it, limited-crafting/knowledge, actor validation,
spectator restrictions, item eligibility or route capability checks. Existing
complete-plan matching protection remains in place until a separately proved
official replacement preserves its exact ordering and deny semantics.

## 3. Native operation and irreversible boundary

The host privately owns `PreparedCraft` for one `CraftOperation`. It is not
cloneable/serializable/reusable. Conceptually the host performs:

1. Validate the real actor, menu/world/block lifetime and action. Freeze current
   recipe/provider generation and acquire an operation-scoped storage authority.
2. Resolve the candidate and prepare one pair against the complete original
   grid. Validate structure/counts, crop-to-original mapping and native value
   representability before retaining it. A provider receives no mutation handle.
3. Privately stage output allocation, one-per-occupied-cell input decrement and
   all remainder placements/merges/drops. Use the full prepared output, not a
   stale result display. Use actual native stack/slot limits and equality rules.
4. Before publication, revalidate the same owner and all affected storage and
   recipe/provider generations, including destination changes and A→B→A edits.
   Exclusive native borrows may supply this guarantee instead of counters; a
   pointer or value comparison alone does not. The host must account for every
   mutation path or refuse the unsupported route.
5. Cross one native commit boundary. Move the retained pair into its staged
   effects exactly once. Never call preparation, `find` or `remainders` again for
   this committed craft. After success, process another quick-move craft as a
   fresh operation against the new grid.

On conflict, cancellation, stale storage, validation failure, or inability to
allocate under that route's allowed destinations, abort before output/input/world
publication. Full inventory alone need not mean refusal: native remainder drops
or Crafter ejection may be permitted destinations and must be staged honestly. Dropping an unused receipt grants nothing.
There is no `Ready → Preparing` or `Committed → Ready` transition. Reentrant
operations on the same owner must be refused or deferred, not interleaved.

The commit boundary cannot call a fallible/reentrant provider between moving
output and consuming input. Native allocation must either be staged and
infallibly installed under the authority, or offer a complete rollback with
proof. Merely running on one thread is not that proof. Existing `Game::give`
mutates storage and may return a leftover; it is not a preflight allocator.

For world effects, stage data/commands first and publish only after the state
transition is irrevocable. If an effect can partially execute or fail after
publication, that route must stay unsupported until its exact semantics are
proved. This proposal is not an exactly-once guarantee across process crashes,
save failures, network delivery or arbitrary external mod side effects.

RNG draws and native hooks during preparation/allocation are observable too.
Copying Menu slots or rejecting a stale pair later does not undo them. The host
must stage/defer these effects or preserve an explicit, tested native draw/hook
order and abort policy; otherwise that route remains unsupported. A provider
may return different pairs on successive attempts, but receives no host-mutation
capability and must not perform irreversible host/external effects. This protocol
does not roll back the provider's own internal counter/cache on an aborted
attempt, nor claim exactly one callback per successfully produced item.

## 4. Preview and remote behavior

Preview is an observation, not an authority receipt. A preview purpose may
derive a disposable pair and display its output, but it never consumes input,
allocates real inventory or creates a commit token. The authoritative action
prepares its own single pair and allocates that pair's actual full output.

A stateful provider may produce a different action-time output than a preview.
The initial contract must explicitly resynchronize that authoritative result,
not silently consume the preview while using new remainders. If the host cannot
provide correct interaction/state-ID behavior, it must refuse that provider or
route. A stricter stable-preview policy would be a separate documented contract,
not an assumed consequence of this pair protocol.

Remote client prediction cannot grant products. Initially, paired custom recipes
should use explicit server-authoritative result synchronization, or remain
unavailable on that route until equivalent safe prediction is proved. Do not
silently send a different packet format, trust client-proposed products, or reuse
an authority receipt after reconnect, menu replacement or world change.

## 5. Actual caller obligations on this baseline

There are four authority/operation families, not just three remainder call sites:

1. **Local normal result click** (`game.rs`, `click_inventory`): `menu_ops`
   bypasses slot 200 for non-QuickMove. Enter the paired operation before the
   direct output transfer. Preserve cursor/full-result behavior, input/remainder
   mapping and native crafted events. A Menu-only implementation misses it.
2. **Menu engine** (`menu.rs`): protect normal pickup, quick move/retries, swap,
   throw and carried-bundle extraction before `try_remove`, `move_to` or slot
   publication. Crafting pickup-all stays excluded. `on_take` may consume the
   already-owned pair but cannot become the first authorization check. Local
   `menu_session` publishes a copied Menu only on success; that one operation
   can contain multiple crafts, so each needs its own pair.
3. **LAN authority** (`game/lan_crafting.rs`): retain authenticated guest/table/
   spectator checks, state-ID validation, exact copy-back and events. The same
   Menu code is also used by prediction, which is not authority. Existing
   `native_prepare_item_craft` can change the next result display after an
   operation. It cannot replace a committed pair or authorize the next take.
   Any late result override must invalidate the affected preview; never pair
   that overwritten output with a retained, unrelated remainder list.
4. **Crafter** (`game/machines/crafter.rs`): include block lifetime/trigger state,
   map finalization, neighbor-container allocation, item ejection, remainders,
   decrement and native events. These are not ordinary Menu cell copies. A
   provider must be refused on this route until the same pair can be committed
   without partial publication. A successful Menu implementation alone cannot
   advertise Crafter support.

No-provider behavior stays on existing native paths, including special maps,
banner/book remainders and event order. For a claimed pair, observable order
must be explicitly specified and tested. Native outputs currently precede
input decrement in several bodies; copying that sequence without excluding
observers/reentrancy is not atomicity. The claimed path needs a native effect
journal or equivalent proven boundary, not a claim that old ordering already
provides it. Unsupported effectful map finalization must refuse before any map
registration, not degrade to incomplete generic item processing.

## 6. PlutoLib / TConstruct / official ownership

- **Official host**: actual actor/storage authority, operation lifetime,
  generation/borrow enforcement, current candidate ordering, limits/allocation,
  native commit, world effects, packets and events. Official constructor/body
  changes would be needed to implement the protocol; none are made here.
- **PlutoLib later adaptation**: one generic provider registry and arbitration,
  stable fact/proposal facade and actual full-stack staging. Reuse the current
  `crafting::products` service; do not turn its value-only receipt into a native
  capability, add a second registry, or reconstruct stacks from facts.
- **TConstruct later activation**: exact Overslime marker/recipe ownership,
  bottle types, capacities/refill arithmetic, tool selection and custom-data
  keys. Its old assembly/remainder wrappers separately invoke `plan`; remove
  them only after this new paired route is proved and connected. No IDs or
  Tinkers rules belong in the official host or PlutoLib contract.

The Overslime source reference is MOD `8c56bb52`: one existing tool becomes the
output; selected bottles leave glass, unused bottle cells return original
count-one clones. Native one-per-cell consumption plus those remainders is
sufficient; arbitrary per-cell consumption, extra output slots and recipe-ID
allocation are unnecessary for this proposal. A different synthetic catalyst
recipe can use the same pair protocol without any Tinkers dependency.

## 7. Why not just extend Mixins local capture?

Actual no-op plans rejected Menu's state constructor, the local result call
and the Crafter result-capture probe at whole-body safety. Relaxing that safety
rule alone neither creates one authority-owned pair nor moves validation ahead
of output extraction. Crafter's expected destructured-local diagnostic was not
reached and must not be reported as its observed rejection.

A stable official prepare/consume contract is preferable to copied broad bodies
or bypassing Mixins safety checks. Any future official implementation needs
official review and explicit authorization to publish it. This MOD-only draft
does not propose a fork, issue, pull request or communication to that project.

## 8. Acceptance and nonclaims

`acceptance.json` lists required native tests and current evidence separately.
The symbolic oracle checks only one-pair transitions, refusal, replay, stale
generations, preview nonauthority and retry separation. It cannot prove a Rust
borrow design, all mutation coverage, real allocation, NBT preservation,
network synchronization, compatibility with other hooks or crash recovery.

No automatic route is currently supported by this proposal. The already-passed
v14 gate validates the existing explicit staging library, not this new contract.
