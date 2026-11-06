# Alternative existing official seams, 2026-11-06

Status: **entry access proved; automatic paired authority still unproved**.
No official edits/outreach, fork access, installed runtime hook or application.
The exact pinned official Horizon/Mixins source is the same 624c/e8 baseline as
our earlier three negative probes. This investigation uses different existing
extension points, not a retry of those rejected interior/state transformations.

## Positive structural evidence

A single isolated plan accepted three noncancellable, no-argument, no-op HEADs:

- `horizon_client_lib::menu::Menu::click`
- `horizon_client::game::Game<'_>::click_inventory`
- `horizon_client::game::machines::crafter::Game<'_>::crafter_dispense`

Probe source is `8c2c5803`. The actual official plan exited 0 in 9.21 seconds,
with minimum free 10,908,336,128 bytes and no disk stop. The companion JSON records
binary/source/plan/log hashes and all exact function fingerprints. No apply or
native compilation followed. There are no argument bindings in this probe, so
it does not prove a borrowed native context or consuming wrapper signature.

This matches the explicit preserved-body HEAD handling in official Mixins
`plan.rs`: ordinary interior-call transformations rewrite the body, while HEAD
leaves it intact. We therefore do **not** conclude that every mod-only operation
wrapper is impossible. A recursive-original/reentry wrapper is an existing
pattern elsewhere in PlutoLib, but a reentry flag by itself is not crafting
ownership or rollback.

## Existing native APIs checked

- `native_prepare_item_craft` (API 290) changes only a result before it is sent.
  Its signature has no paired remainders or consuming operation. LAN callers
  invoke it after menu operations, including when refreshing the next result.
- `native_item_crafted` (API 161) explicitly only observes a take that already
  happened. `native_crafter_craft` (API 205) reports completion after publication
  and decrement. Neither can preflight or roll back a failed paired take.
- `native_anvil_craft_pre` offers an anvil-specific undo/cancellation mechanism.
  The native `AnvilCraft`/`AnvilUndo` fields and caller logic concern anvil inputs,
  output, levels and wear. They are not a generic crafting-grid pair API.
- Server native menus are display-only chest menus: clicks move nothing and are
  delivered to the mod. This is a useful route for a separately designed mod UI,
  but it does not replace ordinary inventory/table/Crafter consumption.
- `native_exchange_items` stages exchange of existing offers between two distinct
  players. `native_move_stack` moves an existing stack between player/ender
  inventories. Neither accepts synthesized products, crafting-grid deltas or
  per-cell remainders. Using fake players/escrow would change their contract and
  is not proposed.
- `native_take_items`/`native_give_items` operate on plain item IDs separately.
  `native_give_stack` preserves a supplied full stack but can return leftovers
  after allocation. Composing those calls is not a single checked crafting
  transaction, and plain IDs do not capture an Overslime tool's component state.

Exact files and hashes are recorded in the JSON. No native API was executed on
real inventory, player data, menus or the world during this investigation.

## Why an outer wrapper is not implemented as a shortcut

A scoped preparation cache around HEAD would still need to refresh the actual
result before extraction, bind to the right owner/authority, and carry exactly
one pair through each successful quick-move iteration. Nested find calls,
previews, late result replacement and remote prediction cannot claim its pair.
Clearing or replacing a context after the result moves cannot undo that move.
The actual original calls also include allowed drop/ejection destinations, RNG,
map registration, hooks and events; copied Menu cells alone do not stage them.

Combining new assembly/remainder replacements with the matching boundary must
also preserve the existing mandatory deny/composition guard. Relaxing its
exclusivity to make a cache work is not evidence of safe composition. No such
change was made. The broader four-family transaction redesign remains possible
research, but no inspected existing API supplies its minimum contract already.

## Bounded work continued instead

With integration ownership agreed, the next shared extraction is the repeated
ordered `Option<bool>` AND/OR fold in recipe conditions, source conditions and
melting ingredients. Its exact unknown propagation and callback visitation can
be checked without claiming crafting authority. Strict station predicates use
different unknown semantics and remain untouched. See `predicate-folds.md` on
the corresponding shared-feature checkpoint; this investigation does not claim
that separate work solves automatic crafting.
