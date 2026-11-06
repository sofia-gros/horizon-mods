# Latest mining scalar contract result: 36 passed

- Source: `6dc08fc3946596ac21a228901b74f9e71d27df96`.
- Trigger: `checkpoint/plutolib-latest-mining-differential-ci-20261106`.
- [Run 42213696048](https://github.com/sofia-gros/horizon-mods/actions/runs/42213696048), job `127878324648`: SUCCESS.
- Official native dependency source: `horizon-rust/horizon` at `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`.
- Unchanged MOD policy/tool source: `08afa992d5091e03360f3473ea93fd17aa793987`.

## Observed evidence

The cold run compiled actual `horizon_core`, `horizon_client_core`,
`horizon_items`, `horizon_world` and exactly one test harness. It enumerated the
sealed binary and executed all 36 tests once, without filters: **36 passed,
0 failed, 0 ignored, 0 measured, 0 filtered**. The existing 24 policy tests and
12 extra differential cases are disjoint subsets of this one execution, with
separate receipts. No prior tests were filtered or replaced.

The 12 extra cases cover incoming speed, environment order, current versus
previous water state, flat/Airborne/armor order, broken and malformed tools,
unknown versus empty tags, queried-position Dwarven policy, harvest division,
and caller-specific zero/NaN behavior. They passed without modifying the frozen
source or expected results. The unchanged TConstruct conditional math retains
its existing `max(0.0)`; the native division fragment is not independently clamped.

All 19 phases ended with exit 0. The 25 controller, 20 preparer and 6 renderer
Python checks also passed. Native compilation took 51.760 seconds. The final
receipt reports `passed=true` and `verified=true`: source, official checkout,
lock/path identities, generated OUT_DIR and sealed binary checks passed.
Minimum free space was 89,840,652,288 bytes, above the preserved floor.

- Binary SHA-256: `a951dd61e696afd551ca79e263e9b0c51f5ba9acf756682769305c4668220f67`.
- Source seal: `33a5385bb998aa5e38b6224e228c9b266e8c8c68f1ebf220c9420ff552343a56`.
- Official seal: `d38e336ef123cf4e6c8f33961b3438bf83ade7fbf849df890de3d32f0b2a1a7b`.
- Generated output seal: `6e6afbffa18630356c746aaebcaa5613af77fce4c6afa4e1d72241337719b566`.

The retrieved job console contained 72,078 UTF-8 bytes, SHA-256
`fd1a06798b3ee5c7b77b2dc7acbb248d55f9cdb9b33ec50888e0cb9564105d27`.
All 36 visible `test ... ok` lines, the strict summary, phase receipts and the
final receipt were checked against one another. Full per-phase raw logs were
transient on the runner; their hashes do not imply a permanent download service.
The compact durable receipts below preserve the observed evidence.

## Scope and remaining gates

This is a finite scalar-policy and actual native Tag/ItemStack dependency proof.
The official arithmetic and completion predicates are source-pinned projections,
not execution of complete Game host/guest paths. In particular, zero speed with
zero hardness yields NaN; the local and delayed-guest comparisons are distinct.
The fixture does not normalize that difference or establish guest permissions.

It does **not** type-check or execute the new Game-side adapter, the full shared
actor/frame policy, complete package composition, input synchronization, Wire
identity, native hook ordering under other mods, or production gameplay. Those
remain separate gates. No ordinary manifest was activated and no merge was made.
The exact CI trigger stays at the tested source; this result is saved only on
the non-trigger source branch.

- [Full final receipt](final-receipt.json)
- [Existing 24 subset](existing24-receipt.json)
- [Additional 12 subset](additional12-receipt.json)
