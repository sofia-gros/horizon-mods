# Plain inventory cell operations

The additive `horizon_client::pluto_stack_cells` native adapter works on explicitly
borrowed `Option<ItemStack>` values. It has no registry, Game reference, native
window index, save operation or packet sender. It is a candidate extraction of
three mechanics in TConstruct's `source-tinker-chests.rs` at `921bb12d`.
The three TConstruct helper wrappers now delegate to it in a separate adoption
commit. Their four caller files, layout and notification order are unchanged.

- `pickup` takes a whole/half stack, places one/a limited stack, merges or swaps
  plain cells. The caller supplies the effective limit and merge predicate and
  has already admitted both taking and placing in that cell.
- `take` removes one or all items from one cell. It returns the stack; it does not
  spawn an entity, dispose of an item or run native drop hooks.
- `collect_ordered` visits explicitly indexed cells in two caller-owned groups,
  first non-full stacks, then full/over-limit stacks. The caller supplies order,
  visible/allowed cells, the carried limit and merge equivalence. Duplicate
  entries remain duplicate visits; out-of-range entries are ignored.

The operations move or clone actual stacks, never reconstruct them through NBT
or a semantic DTO. They alter counts, cell occupancy and ownership only. An
accepted merge keeps the receiving stack's identity. If the supplied comparison
ignores differing opaque backing or runtime fields, those differences are not
preserved when the source stack is exhausted. No general lossless merge or
transport guarantee is claimed. Existing native component equality deliberately
omits retained Wire and bundle selection; snapshot equality is a separate API.

Callers retain layout, slot admission, ChestKind capacities/uniqueness, resolved
item limits, dirty marking, save order, synchronization and native notification
order. The helpers cannot validate actor/world/physical-slot authority. They are
not replacements for native `Menu::pickup`, `pickup_all` or LAN View operations:
those also perform bundle overrides, mayPickup/mayPlace, result-slot behavior,
onTake and slot-change hooks. These helpers run none of those hooks.

## Deliberate malformed-input differences

The narrow wrapper adoption deliberately maps these failures to unchanged/false
or None. These fail-closed corrections are separate from valid-input equivalence:

| Input | New adapter | Existing helper at `921bb12d` |
|---|---|---|
| Zero-count pickup cell or cursor | `ZeroCount`, unchanged | May move/remove zero stacks; a right-click zero cursor can underflow |
| Zero pickup limit | `ZeroLimit`, unchanged | Taking still works; right-click into empty can insert one |
| Zero-count take cell | `ZeroCount`, unchanged | Returns the zero-count stack and clears the cell |
| Zero collection limit/cursor | Error, unchanged | Usually no-op; a zero cursor may collect items |
| Zero-count selected collection cell | `ZeroCount`, unchanged, before policy or mutation | Can remove zero cells while returning false |
| Zero-count unselected collection cell | Not inspected | Not inspected |

Missing/empty cells and positive over-limit stacks are not malformed. Pickup can
extract an over-limit stack, but cannot add to it. Collection retains its two-pass
handling of full and over-limit stacks. A policy predicate must be read-only;
there is no rollback promise for a panicking or side-effecting predicate.

## Evidence scope

The bounded native fixture uses official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`
and its Cargo lock. The first proof compiled the unchanged full TConstruct helper source as the
reference. Adoption freezes the three old function bodies byte-for-byte as an
independent oracle, compiles the actual delegated wrappers and Pluto module, and
uses the single actual inventory API crate. A source guard reconstructs the old
complete file from the three frozen bodies and verifies its original SHA-256,
ensuring all other policy and helper code is unchanged.
No ItemStack/NBT mock or recreated private native declaration is used.

The executed differential matrix contains 2,376 pickup, 42 take and 640 ordered
collection comparisons, including `u32::MAX`, component variants, nested stale
wire, root usable/stale wire, runtime bundle selection, duplicate indexes and
padding. Native Debug values compare the full resulting stack fields, including
private retained baseline values; this is a bounded test observation, not a
production equality API. Separate cases cover malformed inputs and explicit
merge-policy differences. Source hashes are sealed before and after execution.

Four unchanged caller families reach these common helpers:

- Local chest: `source-tinker-chest-click.rs`.
- Local side chest: `source-side-chest-click.rs`.
- LAN chest: `source-chest-guest-click.rs`.
- Workstation LAN: `source-crafting-lan.rs`.

This establishes source reachability and bounded helper semantics, not execution
of those Game/Hub callers, gameplay, save/reload or LAN synchronization. At exact source `538c0fa4813d34ce986670850fae41d8437dea79`, all 30 Rust
tests passed: six new boundary tests, 18 unchanged chest tests and six existing
NBT tests. The six boundary tests include the 3,058 differential comparisons
above. All 6,586 tracked source hashes and strict official dependency identities
were unchanged after execution. The continuous disk guard did not stop; minimum
free space was 11,053,699,072 bytes. See the [machine-readable receipt](plain-stack-cells-validation-20261106.json).
The separate source/package guard suite passed three checks. A complete packaged
plan/apply/type-check has not yet been recorded for this additive slice.


## Narrow consumer adoption gate

The wrapper adoption follows the additive proof; it does not retroactively
extend the recorded `538c0fa4` result. Its fixture compares both direct API calls
and the actual delegated wrappers to the independent old oracle for each of the
3,058 valid cases (6,116 comparisons), and separately verifies wrapper rejection
of malformed counts/limits. The old source receipt is
`tconstruct/tests/rust/plain_stack_cells_legacy.json`. The adoption rerun at exact source
`f2b2cb17a53d1b42bcc263a49596b8eb0bcd9667` passed all 31 Rust tests:
seven boundary tests, 18 actual delegated chest tests and six NBT tests. All
6,589 tracked source hashes and the strict official dependency identities were
unchanged. Minimum free space was 11,051,569,152 bytes; the continuous guard did
not stop. Three cell guards, four projection guards and 11 native packaging
guards also passed. See the [adoption receipt](plain-stack-cells-adoption-validation-20261106.json).

Before that final run, the unsupported `//!` support-module header was corrected
to ordinary comments and checked by the native package's generic header guard.
The first `538c0fa4` fixture result remains evidence only for its earlier bytes;
it never proved official package planning. The final adapter still needs its
complete native-package integration gate. There is no live native-menu, save or
LAN proof.
