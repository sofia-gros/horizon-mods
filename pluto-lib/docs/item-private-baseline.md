# Read retained native item baselines

This additive storage boundary addresses a gap that the existing observable
snapshot API deliberately excludes. On official Horizon `624c4f62`, two retained
Wire values can contain identical bytes, both be stale, and have different
private decoded baselines. A later component change can make one patch usable
without making the other usable. Public patch bytes and current usability
therefore cannot establish private baseline equality.

## Ownership and behavior

- The `wire-read` module is inserted as a public child of native `item_data.rs`.
  It exposes only a borrowed `Option<(&ItemStack, &[u8])>`. It neither writes,
  clones nor serializes private native state and introduces no reverse crate
  dependency.
- The separate `pluto_stack_snapshot` client module adds
  `same_item_with_retained_baselines`. Existing `same_item_snapshot`, all caller
  choices and save eligibility remain unchanged. Only the shared ordinary child
  iterator becomes crate-visible.
- Observable count, component, wire byte/usability and bundle-selection policy
  still has one owner. The new comparator invokes that policy for the root and
  each retained baseline, then discovers further baselines through the same
  ordinary child iterator.
- Completed pairs are memoized after every descendant comparison passes. Raw
  addresses identify already-completed work only. There is no address equality
  shortcut, `Arc::ptr_eq`, mutable access or unsafe code. Native immutable Wire
  construction produces acyclic value graphs; the cache prevents exponential
  repetition in shared DAGs, not the ordinary native traversal's recursion or
  stack-depth cost.
- No caller obtains authority to commit, merge, transport or persist state.
  All retained wire remains ineligible for the lossless saved-item adapter.
  No TConstruct policy or consumer is changed by this additive boundary.

## Gates and proof procedure

The mandatory complete-plan preflight checks the exact official `item_data.rs`
SHA256, the private Wire declaration, reviewed reader bytes, module identities,
locations, visibility and copied source. Additional item-data injections or
state modifications require separate review. The production seam is an ordinary
Mixins child-module insertion, not an edit to the official upstream checkout.

`tools/prepare_stack_snapshot_fixture.py` requires the clean approved official
checkout, an actual successful applied package, its immutable `cutoff.json` and
`stage-source-sha256.json`. It validates every original stage input against the
clean checkout and the complete applied-stage seal. The fixture copies the two
unchanged client helper modules and uses the actual staged `horizon-items`
crate, including its inserted private-field reader. It does not recreate a
private declaration, native ItemStack, Wire or codec. One real packaged inventory
crate is used. Dependencies are seeded from the official lock and may add only
the fixture and packaged API identities.

Run the prepared fixture with `cargo test --locked --offline --lib`, then call
the preparer with `--test-output` to require all eight named tests and with
`--verify` to recheck the complete stage, fixture sources, lock and dependency
identities. Use the coordinated shared target with debug info and incremental
compilation disabled, at most two jobs, and the continuous disk-floor guard.
The full package check separately verifies native surrounding wiring.

The eight real native regression cases cover direct borrowed values; different
stale private baselines with equal public facts; independently allocated equal
values; baseline count, component and runtime bundle-selection changes; all
four ordinary nested-stack fields; recursively retained baselines; shared
baseline DAG traversal; and unchanged save rejection/input state.

The [2026-11-05 applied-items proof](item-private-baseline-validation-20261105.md)
passed all eight real native cases at integrated cutoff `4d82580b`, after the
complete package plan, all mandatory preflight guards, actual apply and lib/bin
check passed. Source-only guards alone do not establish native compatibility.
This proof is not live Game capture, arbitrary mutation safety or full
TConstruct runtime validation. No existing consumer has adopted the additive
comparison.
