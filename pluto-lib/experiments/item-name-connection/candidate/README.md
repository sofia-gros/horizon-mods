# Explicit item-name removal: isolated source candidate

**Not enabled or validated.** This experiment is absent from the normal
PlutoLib package. Its manifest, eight entry-gate model tests and six owned-Tag
tests have not been planned, applied, compiled or executed. The preceding
identity-probe acceptance is not acceptance of this implementation.

Scope: preserve an explicit `minecraft:item_name` removal on incoming native
item patches and on native canonical outgoing patches. It does not add a name
dictionary, aliases, selected-language state, item defaults, author registration,
or automatic crafting. The existing language/Text public and internal layers
remain unchanged.

## Incoming patch

The two previously accepted expression boundaries are the `components_tag`
result in `item_sync::from_network` and `item_sync::template_tag`. The candidate
edits that owned result before native decoding. If the wire patch removes
ITEM_NAME, it removes the added name and records `!minecraft:item_name` as an
empty compound. Removal wins over a contradictory addition. The original
protocol Stack and retained wire bytes are untouched.

The adapter moves the unrelated Compound fields without interpreting them.
Its raw/invalid-UTF-16 preservation assertion is limited to this adapter's
input/output: native `decode_components` subsequently uses lossy string keys
and canonicalizes some values. This is not a general item/NBT losslessness
claim. Unexpected noncompound input is unchanged rather than reconstructed.

## Canonical outgoing patch

The proposed exclusive `network_stack` HEAD first consumes a per-thread
one-shot bypass, before observing an item or invoking any callback. Otherwise,
an explicit removal arms one synchronous call to the same native function with
the same borrowed stack. That immediate entry consumes the arm, continues into
the native body once, then the outer adapter removes any added ITEM_NAME and
emits one removed ITEM_NAME. Native failure/None is retained. The proposal
introduces no ItemStack clone, mutation, saved state or cache.

The guard holds no item identity or player data. After the immediate entry has
consumed the arm, nested charged/container/bundle calls can independently use
the same scope. RAII clears pending state on normal return, return without
consumption and unwinding. A duplicate arm refuses to call its closure; the
adapter then fails closed with None. Panic-abort is process termination, not
recoverable cleanup. This candidate does not authorize arbitrary callbacks
between arming and the immediate native entry.

This is not yet a production-safe contract. A mandatory complete-plan guard
must reject every competing injection into the send target and both receive
targets, as well as rewrites of their selected component producer. It must bind
the exact native files, helper/module bytes, entry ordering and operation
fingerprints, not just the candidate manifest. That guard is **not implemented
in this checkpoint**. Actual codec/compiled composition proof is also pending.

## All native send callers and simpler alternatives

At official Horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0, the complete
Rust identifier search finds four calls in `crates/horizon-client/src/lan/guest.rs`:

| Caller | Line | Role |
|---|---:|---|
| `stack_to_network` | 903 | Top-level canonical conversion |
| `network_stack` | 1153 | Charged-projectile child |
| `network_stack` | 1171 | Container child before native Template serialization |
| `network_stack` | 1200 | Bundle child |

No function-pointer or other direct caller was found in that official export.
This is an exact-source inventory, not a future-version guarantee.

- Wrapping each existing call would be simpler and need no state. However,
  Mixins e8cd0aa38cffef49cb2e088ef716baa107148c02 `plan.rs` 930–952 and
  1101–1119 apply whole-body Safety to WRAP_OPERATION. The recursive function
  contains `?`, closures and calls beneath `&Some(child.clone())`, which the
  rule rejects. This is a source-rule finding, **not a newly executed plan**.
  The three recursive sites cannot be covered by only an outer wrapper.
- An outer postwalk over protocol Stacks would avoid reentry but must also
  decode and re-encode the already serialized CONTAINER Raw payload, preserve
  exact source/output correspondence and account for codec failure/budgets.
  That is a different, unproved transformation and is not implemented here.
- Reusing the private retained-Wire owner as mutable state is not allowed:
  Wire is a read-only preservation/baseline capability, not a mutation token.
- Cloning the root and stripping the marker before reentry was rejected.
  Marked ancestors each clone their remaining subtree: a chain can add O(N²)
  clone work. Native protocol input has MAX_NESTING=64, but locally constructed
  ItemStack trees and canonical send have no equivalent universal bound.
  This candidate removes that additional cloning, but adds call/RAII frames at
  marked levels and does **not** establish a new depth or allocation bound.

## Raw-Wire route is deliberately separate

`Game::to_network` can return a usable retained raw patch before calling
`stack_to_network`. This proposal neither mutates that Wire nor forces this
fast path through canonical encoding. Untouched/count-only raw echoes retain
the exact original patch, including contradictory added+removed ordering.
After an edit invalidates the native baseline, canonical send instead applies
the explicit-removal semantic rule. The raw bytes may differ while native
decoded naming agrees. The existing permission checks remain native.

## Required proof before activation

1. Independently review gate one-shot behavior, thread isolation, unwinding and
   no callbacks before consumption; run all eight actual gate model tests.
2. Compile the actual candidate modules/helpers with actual official
   core/items/protocol types. The six Tag tests are not an ItemStack proof.
3. Verify actual receive/decode/Wire-baseline construction, raw usable echo,
   count-only echo and canonical edited send separately. Do not substitute
   semantic mock ItemStacks for the native codec.
4. Test root/charged/container/bundle and mixed nested markers; native body
   and registry callback counts/order; no-marker equivalence; None; unrelated
   additions/removals; duplicate/contradictory name patches; retained unknown
   wire bytes on the raw route; canonical route limitations explicitly.
5. Test panic before/after consumption and nested panic, repeated successful
   conversions after unwind, and independent concurrent threads with no
   pending state after terminal outcomes.
6. Plan/apply/type-check the exact complete package under the mandatory guard.
   Keep any default-component storage and actual Game display claim separate.

Source-only review of the receive adapter found no serious owned-Tag issue.
No model, native, networking or gameplay success is claimed for this candidate.
