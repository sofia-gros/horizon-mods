# Remaining inventory insertion and drag boundaries

Read-only source audit, 2026-11-06. Reviewed MOD cutoff
`48bcf882050c21e24b468f4dfdaeda4811052d3f`; the two consumer files below are
byte-identical to central `b19b35a10b96e398ca8dabcb46b200d88950edaf`.
Native comparisons use official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`. No implementation, native plan,
test or live interaction is claimed by this audit.

## Concrete reusable candidate: ordered insertion

`tconstruct/mixins/tconstruct/transaction.rs::source_give_inventory` at line 1453
merges into the selected slot first, then other existing slots in ascending
order. It fills empty slots in ascending order. `stackable` independently
enables the merge pass. The caller provides the limit and handles the remainder.
Actual callers include local/guest Part Builder results, station results,
fluid-container returns and Smeltery insertion.

The same file's `source_quick_move_player` at line 1514 validates at least 36
slots, a positive limit and a positive incoming count. Both its merge and empty
passes visit hotbar 8 through 0, then inventory 35 through 9. It uses
`source-tinker-chests.rs::insert_slot` with `ChestKind::Plain`; that helper already
delegates quantity planning to Pluto's `plan_move`. Local/guest chest, side-chest,
Smeltery and workstation routes use this insertion order.

These share a useful mechanical boundary: insert an incoming whole native stack
into explicitly ordered plain cells, with separate merge and empty orders and
caller-selected merge policy/limits. It would not choose player layout, include
offhand implicitly, drop a remainder, interpret a recipe or publish storage.
The existing single-destination planner already covers the count calculation;
another arithmetic planner is unnecessary. The potential addition is the native
ordered application adapter, not an ownership or commit abstraction.

Before implementation, freeze both complete old helpers as independent oracles.
Compare every valid result and full native stack value, including opaque/runtime
fields, with unequal merge and empty orders, disabled merging, short/extended
inventories, full/over-limit cells, partial remainders and large counts. Preserve
the existing receiving-stack identity on an admitted merge; native component
equality is not opaque identity. The adapter must not inherit stronger authority
from a snapshot comparator.

Malformed-input behavior is not currently uniform. `source_quick_move_player`
returns a zero-count input unchanged. `source_give_inventory` has no initial
zero-count check and can insert a zero-count stack into an empty cell before
returning no remainder. Both reject a zero maximum. Existing zero-count
destination cells may be topped up by these helpers. Any normalization or
fail-closed rejection must be identified separately, not called exact parity or
silently inherited from the newer plain pickup API.

`source_quick_move_to_builder` retains its pattern-versus-material selection and
input-before-side order. `source_quick_take_part_plan_inner` additionally owns
Part uniqueness, staged recipe inputs, random/remainder handling and final
publication. They are not candidates for blanket replacement by one traversal.

## Drag distribution is not native menu dispatch

`source-tinker-chests.rs::quick_craft` at line 164 is already the one shared
TConstruct implementation used by local chest/side/workstation routes, guest
routes and read-only drag preview. It deduplicates caller-mapped cells before
division, filters against current count/component/limit rules, and rechecks Part
uniqueness through `insert_slot` after each earlier insertion. The dynamic
admission matters: two initially empty Part cells do not necessarily remain
independently eligible after the first insertion.

The remaining generic-looking arithmetic is interleaved with selected-cell and
creative policies. A direct native replacement would not preserve the contract:

- Official `Menu::click` quick-craft (menu.rs lines 2709–2793) owns drag phases,
  may-place/drag hooks, native setters and single-slot fallback to Pickup.
- Official LAN `quick_craft` (lan/menu.rs line 791) owns View admission and its
  own single-slot fallback and count-limited placement loop.
- TConstruct consumes already mapped selections, applies its own capacities,
  and also runs this calculation on copies for preview. Its creative branch
  can continue offering per-cell quantities after its remaining count reaches
  zero. This is existing policy, not a newly approved native-behavior change.

Do not extract a stateful drag registry or substitute native drag semantics on
the strength of this audit. A future pure distribution service would first need
an explicit contract for dynamic revalidation, creative quantities and preview
versus mutation. Merely factoring the division and deduplication is not a new
host capability.

## Existing native insertion is not a drop-in replacement

Official `Game::inventory_add` (game.rs line 3345) includes offhand between the
selected slot and the rest of the 36 player slots. TConstruct's reviewed helper
does not. Official `Menu::inventory_add` (menu.rs line 2044) separately handles
damaged damageable items, creative disposal and repeated resource insertion.
Reusing either wholesale would change caller policy, despite similar names.

The existing machine publication audit remains applicable: in-memory dirty-data
updates, native notifications and held/inventory publication are distinct phases.
No new persistence/synchronization authority is justified by these insertion
loops. Keep all current caller saving, dirty marking, notifications and refresh
order outside any eventual mechanical adapter.

## Source identities

| File | SHA-256 |
|---|---|
| transaction.rs | `7ac6ccb9a153728f2d7cf18ff75b8256c250f9c59af5a2808bd83d860e16816b` |
| source-tinker-chests.rs | `9a9c8179cfdd78e63cb628de301a0e1a6ae30b6a0e61c9bd0d9805371bd387c1` |
| official menu.rs | `d1c87e41131b3338b13201f362eca52790440a25536a8d8495413d5e15eb4c36` |
| official lan/menu.rs | `709f2042df914eb5080e34344080bfb262d9a44857a6ef3a4327cb7b944ebd0a` |
| official game.rs | `cdfc9247cce139c00e5a36266812c7f7f42ccbb565d67705a896520b4b62b22c` |
