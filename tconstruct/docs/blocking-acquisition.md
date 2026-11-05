# Blocking acquisition on ordinary tools

This #379 slice stacks on Parrying acquisition #940 and the stock-shield /
Blockade work beneath it. It accepts the exact canonical Blocking ability
recipe: one steel ingot and four planks in five occupied ingredient slots.
The ordinary recipe therefore needs the five-input Anvil. A matching modifier
crystal can use the existing three-input Station path.

The live target predicate is the intersection of charge-interactable and
durability tools, excluding parry tools and shields. All four tag snapshots
must exist, including exclusion tags; missing is not treated as empty.
Steel and plank ingredients use the resolved item tags, with no substitution
or invented steel supply. The target still needs to be supported by the
existing assembly/normalization transaction.

Blocking consumes one ability slot and allows one purchased level when no
material Blocking is already present. It is counted in part-swap slot
revalidation. The full highest-priority dynamic Blocking definition must match
the pinned source module, unlike the separately static Parrying registration.
Changed recipe/module semantics, malformed saved levels, unavailable traits,
stacked tools and zero-count inputs fail closed.

The saved level can be extracted with a Sky Slime Crystal and Wet Sponge,
returning its modifier crystal and a Dry Sponge. Extraction uses the same live
target predicate and canonical definition. It cannot remove the last
Blocking/Parrying prerequisite while any saved or material Blockade remains,
including partial pane progress. The existing selection IDs are retained by
appending Blocking; item and Station views use the translated modifier name.

## Runtime and validation boundaries

This is acquisition, not ordinary-tool blocking runtime. The inherited stock
Travelers/Plate/Battlesign shield implementation is unchanged. Use priority,
start/stop behavior, amount/angle, use speed, durability and cooldown on
non-shield tools still need a separate runtime slice. Parrying's inherited
offhand attack and 20-tick block action remain separate too.

Three regression tests were added but not run. Brief static review covered
canonical recipe/module equality, live inclusion/exclusion tags, five-input
and crystal routes, slot/trait caps, extraction dependencies and append-only
selectors. Rust formatting parse and diff whitespace checks only; no tests,
build, planner, CI inspection, version checks or merge. Actual steel supply,
GUI, save/reload and LAN remain unverified.

Source: Tinkers' Construct f4e77d43, Blocking ability/salvage recipes,
`blocking.json`, charge/durability/parry/shield item tags and `BlockingModule`.
