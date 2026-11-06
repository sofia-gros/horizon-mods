# Quantity planning for temporary fluid-container slots

Status: source prepared; native differential tests are **not yet run**.
This is a separate consumer-only follow-up, not part of the frozen v15 proof.

The existing local and guest Smeltery menu paths use
`source-fluid-container.rs::click_with_color` and `quick_input_with_color`.
Both now delegate only the admitted incoming quantity to PlutoLib's existing
`plan_between`. No API, registry, native hook or notification is added.

## Policy remains with the consumer

- The output slot rejects inserted items, and incompatible input stacks never
  swap. The index gate runs before taking the cursor or querying fluid admission.
- Empty-cursor extraction is byte-identical, including its existing handling of
  a zero-count saved stack. It does not use the stricter plain-cell pickup API.
- `accepts_with_color` runs once at the same point with the same `known_fluid`
  callback. Local color permission and guest restrictions remain unchanged.
- Existing item-family capacity selection is unchanged: 1, 16 or 64.
- Equality remains the exact existing item/damage/extra expression, not a new
  wire/snapshot policy. An admitted merge retains the receiving native value.
- The existing mutation statements and order are retained after quantity
  selection. No live save, held-item update, menu publish or LAN code changes.

The admitted incoming stack is positive by the unchanged parsing branches:
tool capacity requires count 1; can, potion and lantern parsers reject count 0;
the ordinary fallback requires count greater than 0. This matters because the
shared planner refuses a zero source. The source guards pin every parser and
all other code by restoring the two old helpers and hashing the complete
original source. The planned native cases also exercise each successful family
and its zero-count rejection. Existing zero-count destinations remain eligible
under the old policy; they are not normalized or rejected by a stricter adapter.

## Independent fixture

The complete old two function bodies were frozen before editing, with their
source/function hashes. The fixture compiles the complete actual current
fluid-container file as an exact byte prefix and appends test-only nested
modules so the original bodies can borrow the real private parser functions.
No native ItemStack, NBT, potion catalog, fluid parser or machine state is mocked
or recreated. The actual native support modules and one shared inventory API
crate are used with clean official host path/lock checks.

Five named tests are prepared:

- 19,440 click comparisons and 3,240 quick-input comparisons across item families,
  zero/high counts, capacities, incompatible values, malformed components,
  output/invalid indexes, right clicks, color permission and fluid availability.
  Each compares the return value, complete native debug values, and exact order
  and number of `known_fluid` calls.
- 540 empty-cursor comparisons, plus empty/missing-input cases.
- Zero-count rejection across all sampled admission families, with no fluid
  callback, plus successful cases for each distinct parser branch.
- Two opaque-wire receiving-value comparisons, preserving the existing native
  equality behavior rather than claiming a stronger lossless boundary.

Six Python source/dependency guards pass, including adversarial policy/parser
changes. Rust source parsing passes; no native execution is claimed by this
checkpoint. Full TConstruct, menu lifecycle, save/reload and LAN behavior remain
outside this focused quantity-only proof.
