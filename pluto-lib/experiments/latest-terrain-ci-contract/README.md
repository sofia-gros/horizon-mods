# Latest terrain profile CI preparation

**Pure contract code only, not a runnable CI or a compatibility result.**
There is no workflow, command launcher, fetch, apply, compile or filesystem
mutation in `contract.py`. Twenty-five Python parser/source regressions pass on synthetic
strings. Those strings are not compiler output or Rust results. The latest
complete plan and its actual rendering identities remain unavailable.

## Frozen inputs and missing admission

Latest official sources are Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`
and Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`. The preceding 600-second
diagnostic ended with `E_LOCAL_NOT_FOUND` in an old TConstruct mining hook;
it did not produce an accepted complete plan. The mining repair's reduced
formal-entry probe cannot supply whole-composition or rendering compatibility.

The prepared source composer is checkpoint `eb232139e7f26c5d41a2580013a285732960b0e0`.
It preserves historical production `43f17427`, the original full-body native
oracle and fixture `1ac143d8`, adding the unchanged profile and reload source
fragments in memory. Their execution remains pending. The separate
`terrain-local-bounds` optimization candidate is **not** part of this schedule.

The descriptor intentionally has null package, current guard and accepted plan
admission fields. A later reviewed runner must require all three exact values;
it must not fill them automatically from witness JSON, accept the historical
624c proof as current, skip an old guard failure, or invent new node identities.
Collector `5b87b28d` belongs after complete plan success and before existing
guards. Whole-plan SHA/bytes must be saved before invoking it, even if its
source/owner verification rejects. A witness remains observation, not approval.

## Prepared execution order, not execution authorization

After latest package/guard admission is resolved, the proposed runner must:

1. Admit an exact public repository push on a dedicated standard-runner branch,
   `contents: read`, checkout without persisted credentials, no cache/artifact
   upload, no billing/permissions change or extra workflow trigger.
2. Apply the already reviewed full safety controller, continuous writable-
   filesystem disk floor, setup reserve, supported visible memory hierarchy,
   bounded logs/timeouts and own-PGID teardown. Keep all missing/failed gates in
   a false final receipt. This module does not implement a safety controller.
3. Fetch only the exact two official repositories/revisions, verify their locks,
   export and full source/file/link seals, verify the accepted production
   payload and latest reviewed guards, compose the source in memory, prepare
   the original synthetic resources and require full inverse reconstruction.
4. Plan, preserve plan SHA/bytes and selected witness, run every applicable
   complete-plan and fixture guard, then apply. Failure stops before compiling.
   No guard metadata may be regenerated as a side effect of this run.
5. Use separate fresh target directories for opt0 and opt3. Each profile retains
   debug assertions, overflow checks, debug=0, incremental=0, codegen-units=16,
   lto=false and jobs=1. Require actual renderer `--test` rustc invocation and
   non-Fresh Cargo artifact profile, source and output path. Full compiler and
   artifact lines must be hashed and retained in the compact receipt.
6. Enumerate each actual test executable with `--list --format terse`. Compare
   the whole sorted line stream, reject extra text/duplicates/benchmarks and
   require exactly the three known tests in the fixture namespace. Other real
   native tests remain in the inventory. Derive each exact filtered count from
   that executable's inventory, not a stale hardcoded count.
7. For each profile run `no-provider`, `registered`, `profile`, and `reload` as
   **four fresh processes**, with `--exact --show-output --test-threads=1`.
   Never reset or reuse the global frozen provider registry between modes.
   Pass the external actual profile label and 64 legacy timing iterations.
   Both original end-to-end modes and all existing correctness/replay assertions
   remain required. The original material-contract tests are a separate exact
   gate. The eight-row schedule does not replace those type/material gates.
8. Verify binary bytes, generated OUT_DIR contents, staged source/lock,
   resources, original inputs and complete ignored/untracked checkout state
   before/after all executions. Execute sealed binaries directly, not Cargo
   test commands that could silently rebuild. Publish bounded exact cases,
   timing arrays and phase SHA/bytes; keep failure tails and missing phases.

The old runner and fixtures are unmodified. The future runner still needs its
current package preparation/guard admission, execution controller and source/resource final verification. Do not call
this checkpoint a ready-to-trigger workflow.

## New strict result contracts

The profile parser requires twenty unique family/callback/lighting tuples,
five replayed sections per tuple, one positive observed clock interval, three
paired comparisons, five microkernel arrays and three native-fallback replay
receipts. It recomputes median ratios and short-sample flags from bounded raw
u64 arrays and rejects duplicate keys, NaN/infinity, missing/extra cases, wrong
scope/labels/counts, weakened same-output assertions and incomplete completion
markers. Short observations remain valid but explicitly inconclusive, including
zero-duration samples. No speed threshold is used.

The reload parser requires six exact ordered cases and one passing actual
test. They test synthetic native owner values, not Game reload/streaming or GPU
display. The compiler/artifact helpers validate the requested profile fields
and source/CWD/path shapes. At rustc's pinned defaults, absent opt-level means
0, debug assertions follow opt0, and overflow checks follow debug assertions;
the actual Cargo artifact must independently report both enabled. Unknown
compiler provenance or missing executable/source seals cannot be established
by supplying strings to these pure helpers. A future controller must enforce
the complete compiler invocation/environment and verified Rust 1.95 toolchain.

Pair timings compare constant fixture callbacks and include adapter/native
emission together. Microkernel numbers must not be subtracted to invent a
separate emission cost. Opt3 can specialize fixed-shape code, so results are
not a generic caching benefit or shipping performance claim. Keep original
end-to-end data beside paired measurements, including unfavorable or noisy
results, and keep CPU synthetic results separate from GPU/live-game behavior.

## Original end-to-end receipt preservation

The pure legacy-mode validator now requires the original five timing cases
(across two processes), four no-provider and fifty-one registered fallback
replay receipts, exactly one resumed claim at the dense-budget replay, all five
registered case markers, and all thirty-two ordered material-fallback input
tuples. It checks complete schemas, scalar types, finite positive timing
samples and recomputed ratios. Missing, duplicate, malformed or cross-mode
markers are rejected. These receipts do not replace the frozen Rust full-output
and scratch assertions, or the post-execution source seals.

The number and ordering are derived from the pinned original fixture. No
production or Rust fixture bytes changed, and no new Rust/CPU run occurred.
Claimed-mixed and secondary-material still compare altered geometry to the
original native geometry; their ratios are not isolated adapter overhead.
A zero-duration original timing is rejected rather than producing a ratio.
The separate profile microkernel parser still accepts explicitly inconclusive
zero-duration samples under its own schema.

A read-only parser audit of the already retained historical log from
[run 42192378071](https://github.com/sofia-gros/horizon-mods/actions/runs/42192378071)
(78,798 bytes, SHA-256
`4a746fa27f7b89ba01cd1248abf20d440c58e334a2b24c843dc6d5f798784e21`)
confirmed the 4/51 replay split, resumed index 18 and exact 32 fallback tuples.
Only GitHub timestamp prefixes were removed for that array comparison. Its
old compiler profile label and evidence scope were not relabeled or used as
a latest-cutoff execution. The 25 committed regression tests use synthetic
strings and do not depend on that private local log being present.

Independent source review caught a profile-parser arithmetic edge: Python's
integer ratio and Rust's separate `u64 as f64` conversions can round differently
above 2^53. Both legacy and profile validators now explicitly cast each median
before division. A regression with medians 2^53+1 and 2^53+2 rejects the other ratio.
This corrects the pure checker only, not a recorded native timing result.
