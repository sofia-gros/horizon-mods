# Choose a storage operation

PlutoLib provides typed Rust operations and official-host adapters. It is not
a loader or a replacement for the host's inventory lifecycle. A mod that uses
PlutoLib routes the supported shared operations through it; other Mixins mods
do not have to depend on PlutoLib. TConstruct owns its own machine schemas,
recipes, capacities and notification order.

Start with the operation you actually need:

| Need | Existing entry point | What remains yours |
|---|---|---|
| Decide how many items fit | `inventory::plan_between` or `inventory::plan_move` | Identity, destination admission, limits and applying the plan |
| Build proposals for multiple storage partitions | `inventory::stage_updates` | Independent copies, schema validation, fresh authority and commit |
| Compare a present generic value | `snapshot_matches` | Equality coverage and protection against changes after the check |
| Stage an actual native item for saving | `pluto_stack_storage::encode_item_lossless` | Original raw-record validation, slot policy and persistence |
| Validate a saved native item Tag | `pluto_stack_storage::decode_item_canonical` | Empty-slot forms, aliases and other schema rules |
| Replace selected fields in a native Compound | `pluto_nbt_storage::patch_compound` | Owned keys, values, aliases, nested patches and eventual write |
| Manipulate already-admitted plain native cells | `pluto_stack_cells::{pickup,take,collect_ordered,insert_ordered}` | Layout, merge policy, permissions, native hooks and notifications |

## A thin public entry, one implementation

New consumers can import the common pure operations together:

```rust
use pluto_inventory_api::inventory::{
    plan_between, plan_move, stage_updates, MoveError, MovePlan, StackSnapshot,
};
```

The `inventory` module only re-exports the existing items. It adds no wrapper
type, algorithm, validation order, registry or commit operation. Root imports
such as `pluto_inventory_api::plan_between` and the original
`pluto_inventory_api::storage::stage_updates` remain supported. Their types,
public struct literals and error values are the same types, not converted DTOs.
Existing `FnOnce`/`FnMut` callback contracts and result meanings stay unchanged.

This separates the mod-facing choice of operation from the implementation
without duplicating it. Host-dependent item codecs and native cell operations
remain in their existing adapter modules; the pure crate cannot import the
native host back into itself.

## One API instance, explicit imports

The host-independent crate is `pluto-inventory-api` (Rust import
`pluto_inventory_api`). The installed native package re-exports that same crate
as `horizon_client::pluto_inventory`. Use the path available in the layer you
are extending; do not copy the crate's modules or registries to make an import
work. Current Mixins packaging restrictions still apply. This is not a promise
of a stable host-independent ABI for every adapter.

Planning and proposal staging are ordinary function calls. They do not need a
registry or event subscription. Use registration or callbacks only where the
specific documented feature actually owns those capabilities.

Native adapters remain host-qualified: for example,
`horizon_client::pluto_nbt_storage` and
`horizon_client::pluto_stack_storage`. The installed package exposes the same
pure entry as `horizon_client::pluto_inventory::inventory`. This is still one
crate instance. No new registration mechanism, transaction token or storage
ownership type is introduced.

## Typed planning and proposal example

The [public-entry value example](../examples/inventory/stage_proposals_public.rs) imports
the actual `pluto_inventory_api` crate. It uses consumer-defined `Variant`,
`Bin` and `Cell` values, not simulated native ItemStack/Game declarations:

1. Borrow typed source and destination identities in `StackSnapshot`.
2. Supply the destination's resolved limit, admission and merge policy to
   `plan_between`. Its indexes are logical 0/1, not physical slot authority.
3. Convert the chosen count into consumer-owned changes.
4. Have `stage_updates` load independent row copies and apply only to those
   proposals. The example keeps both original rows unchanged.
5. Stop. Actual native revalidation, commit and post-write notifications belong
   to the consumer's supported lifecycle, not this demonstration.

The file also shows a rejected second change: staged values are dropped while
the originals remain unchanged. This does not roll back external callback
effects, native RNG, hooks, writes or packets. Returned proposals are in key
order; that is not a directive to change the consumer's commit order.

The [legacy-import example](../examples/inventory/stage_proposals.rs) is retained
unchanged. The public-entry example changes only that import, leaving its
consumer policy, six assertions and required-success checks identical.

**Validation status:** the public entry and both examples passed the exact
[pure-API follow-up CI](https://github.com/sofia-gros/horizon-mods/actions/runs/42177564770)
at `7094f48e01dc17de65e0c0aa27eae418f07ec847`. Four compatibility tests cover
old/new type identity, planner results and callback order, logical indexes,
and detached staging. They are included in the actual inventory crate's
147 passing library tests, not four additional tests.

An external consumer depended on both complete real API crates. Both storage
examples were type-checked, built with observed rustc invocations and executed
separately with exit zero, reaching six assertions each. No copied API, stub or
disabled module set was used. Full source, file-set, manifest, lock and generated
input seals stayed unchanged. Four Python source guards also pass, separately
from Rust verification. See the [exact storage evidence](inventory-public-entry-validation-20261106.json).

The earlier `2ee73a7` CI covered the legacy example against a 143-test inventory
API, not this public entry. The frozen example headers record their pending
status at introduction; these later results supersede that status without
changing their executed source. This is pure-crate verification, not a native
package apply, live inventory commit, save/reload or notification test.

## Keep the boundaries separate

- A move plan is not an item snapshot. It records counts and a proposed move,
  not an immutable actor/world/slot capability.
- Native semantic merge equality may ignore retained Wire and runtime bundle
  selection. The caller chooses whether merging those values is acceptable.
- `same_item_snapshot` checks observable state, including exposed nested Wire
  facts and bundle selection. It does not inspect private retained baselines.
  The separate experimental baseline comparator is read-only too.
- Save eligibility rejects retained Wire recursively. A stack can match a
  snapshot yet be ineligible for that codec. Canonical Tag equality cannot
  recover information lost before the Tag reached the decoder.
- Compound patches preserve untouched exact JavaString keys and values. They
  do not decide whether an alias is valid, own a machine's schema or save it.
- Plain-cell helpers do not dispatch native menu/result-slot hooks. Their
  documented zero-count/zero-limit failures must not silently replace a
  consumer with different legacy behavior.

Do not wrap a sequence of callbacks in a new class and call it an atomic
transaction unless the native operation genuinely provides that authority.
Existing TConstruct `write`/`write_many` dirty-state updates are not proof of
durable disk persistence, network delivery or rollback of prior side effects.

## Detailed contracts and evidence

- [Planning, codecs and snapshot comparison](inventory.md)
- [Compound patches](nbt-storage.md)
- [Plain-cell mechanics and explicit legacy differences](plain-stack-cells.md)
- [Shared API ownership and packaging](shared-api-boundary.md)
- [Outer-tank consumer delegation](outer-tank-patch-adoption-20261106.md):
  separate source checkpoint; its new native fixture is still unexecuted.

Contract examples, source guards, focused native tests, complete package
composition and live game/save/LAN behavior are different evidence levels.
This guide does not upgrade one into another or mark original PRs repaired.
