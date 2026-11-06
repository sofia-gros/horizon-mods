# Terrain CPU cost isolation: proposed next experiment

**Design only. No new profiling module, workflow, native build or timing run is
implemented or executed by this checkpoint.** Publication of this plan does not
start Actions. The main/common baseline has priority; execution requires a
separately reviewed immutable runner and explicit serial-slot release.

## Preserve the successful result

Keep production [43f17427](https://github.com/sofia-gros/horizon-mods/commit/43f174273cd5f1b9cf3acd1d26abab8838025510),
archive `cd6de18cacbe7c40218ec817671bf3842aef3be0d586c613bb788babafbd3a76`,
the successful fixture [1ac143d8](https://github.com/sofia-gros/horizon-mods/commit/1ac143d840eaf3dde9e108ada3f5f2ec9f4c451f),
and its [2c78ab51 evidence](https://github.com/sofia-gros/horizon-mods/commit/2c78ab519fc7411bc3ff83aaa0ce1616f4c0b36f)
unchanged. Do not replace a failed assertion, weaken replay equality or relabel
old data as a new optimized result.

The previous opt-level-0 fixture measured native full-height versus claimed
half-height cells and included test assertions in the geometry callback. Its
1.8855/1.9265 claimed-path ratios are valid measurements of those workloads,
not isolated dispatch overhead or a production performance regression verdict.
The new measurements must retain those end-to-end results alongside the new
controlled comparisons, not supersede or conceal them.

## Two profiles, explicit compiler evidence

Use the same official Horizon 624c4f62 and Mixins e8cd0aa3, Rust 1.95.0 and locked
dependencies as the successful run. Prepare/apply the same production archive
plus an excluded test-only extension. Verify all 12 production guards and a
strict extension guard before apply; no main/consumer/runtime hook changes.

Compile the real renderer unit-test crate twice into separate fresh target
directories, with test opt levels **0** and **3**. Keep debug information off,
incremental off, one build job, debug assertions on and overflow checks on for
both. Pin codegen-units to 16 and LTO off for both new runs to avoid changing
those controls implicitly. These new explicit controls mean the old timing is
historical context, not a byte-identical compiler-configuration replay.

Require the exact renderer Cargo JSON artifact, its profile fields and actual
rustc invocation: crate/source/`--test`, optimization level, assertion/overflow
settings, codegen units, compiler identity, features and link/codegen inputs.
Verify relevant dependency compiler settings too; an environment variable or a
human-readable label alone is not evidence that the configuration was used.
Reject a profile mismatch or a Fresh-only build. Do not call opt3 with assertions
enabled a shipping release profile or game/FPS measurement.

## Measurement matrix

For each profile, run the original two exact CPU modes in separate processes,
plus the existing two native material contract tests. Preserve all original
case assertions and 55 replay receipts. Their five end-to-end timing rows stay
visible under the new profile label.

A third CPU process would run a new exact named test,
`same_output_callbacks_and_microkernels`, using the real synthetic loaders,
assets, tables, chunks, Mesher and output types already in the fixture.
It registers all callback variants before the first mesh and never modifies the
frozen registry afterward. Different comparison fixtures retain distinct actual
resource-owner triples, prepared from identical input bytes.

Proposed callback pairs:

| Family | Callback variants | What comparison can show |
| --- | --- | --- |
| Geometry | original callback including its neighborhood assertions; equivalent build callback without test-only assertions; equivalent immutable fixed-batch clone callback | Cost of fixture-only checks, then construction versus cloning for this constant six-face half-height shape |
| Secondary material | original build callback; equivalent fixed-batch clone callback with the same secondary-slot choices | Construction versus cloning with the identical material, tint, orientation and native emission path |

The fixed batch is a **fixture-only constant** for the explicitly admitted
variant and orientation. Initialize it before timing. It is not a proposed
global cache or a valid optimization for arbitrary world-dependent callbacks.
Unsupported context must not silently use the constant.

Before any timing:

1. Compare direct callback batches/slot selections for the exact admitted
   contexts, using the real prepared binding/neighborhood facts.
2. Compare complete unsorted and sorted `SectionMesh` outputs bitwise, all
   existing auxiliary fields, sprite animation registration and scratch/light
   observations across variants. Use the same meaningful cases and lighting
   options as the original oracle where they apply.
3. Require equal emitted solid/translucent vertex counts and identical material
   layer/UV/tint facts. This is an output-equivalence gate, not merely a matching
   bounding box or test count. Allocation capacity remains diagnostic only.
4. Reuse each Mesher/output/scratch for subsequent ordinary and empty sections;
   retain the original stale/invalid/budget whole-output fallback proof from the
   original modes. Do not bypass those checks to simplify a benchmark.

Only then time the variant pairs. Native-versus-claimed-height timings remain
separately labeled; equal-output comparisons are callback-versus-callback,
never assumed equivalent to the native full-height output.

## Distinguish construction, validation and native emission

Add bounded bulk microbenchmarks of the **actual production pure functions**:
geometry build, fixed-batch clone, cell-local bounds validation, and UV subset
validation with both an admitted and a rejected rectangle. Use black-boxed
inputs/results and the same shapes as the end-to-end test. Confirm expected
results outside the timed interval. These are pure-function timings; they do
not include owner checks, map lookups, native model compatibility or emission.

Do not pretend that subtracting those medians from total time measures
`emit`. Inlining, cache behavior and optimizer elimination make that arithmetic
invalid. The callback comparisons leave the actual adapter validation and
native face emission together. Report that remainder as **not isolated**.
If these first measurements are insufficient, a later separately approved
test-only instrumented adapter could time those phases, but it must have an
exact inverse transformation to the production source and quantify timer/counter
overhead. That instrumentation is not part of this initial plan.

## Sampling and receipts

- Start with seven alternating-order samples, eight warmup mesh calls and 64
  iterations per sample, matching the original sampling shape. Record all raw
  durations, configuration and order, not only a median ratio.
- Record total sample durations and clock resolution/availability. If optimized
  samples are too short for meaningful comparison, report insufficient timing
  evidence and review an increased fixed iteration count; do not auto-tune one
  candidate until its result looks favorable.
- Equality is mandatory; there is no arbitrary speed-pass threshold. Slower,
  equal/noisy or inconclusive outcomes are valid findings.
- Execute the sealed native binaries directly in fresh processes. No Cargo,
  setup, asset loading, registry registration, assertions of equivalence or
  fixture construction belongs inside a measured mesh loop.
- Keep a compact strict receipt with exact test inventory, per-profile actual
  compiler records, all equality cases, every sample, binary/source/lock/resource
  seals and failure phase. Preserve all failed attempts. Raw verbose logs remain
  locally checked in the CI runner with hash/byte receipts, not uploaded, unless
  a later separately reviewed storage policy says otherwise.

## Source and execution boundaries

The proposed preparer should derive an extended test module by one exact
insertion in the frozen fixture. Its inverse must reproduce every original
fixture byte and the native full-body oracle. It must not edit the checked-out
successful fixture. The strict verification of an extended overlay needs its
own metadata and extension guard; running the old verifier against changed
module bytes and ignoring its failure is not allowed.

Before any trigger publication: freeze the new fixture/preparer/runner/config,
enumerate exact expected test names/counts, perform bounded source/parser tests,
and obtain independent review. The runner is then a dedicated public-standard
Actions job, contents-read, exact branch/head and official URL/revision allowlist,
without secrets, permission/billing changes, cache or artifact actions. No
unrelated workflow should run on that push. Remote jobs are serial; this plan
does not reserve or start the next slot.

All setup and output filesystems must retain the existing 10 GiB + 128 MiB
continuous disk floor and setup reserve. Separate targets increase cold codegen
cost; actual runner capacity, memory-limit visibility/admission, timeout and
bounded own-process-group teardown must be reviewed before execution. Never
use the constrained local machine as a substitute for an ungranted CI slot.

No new result from this plan establishes GPU display, real-game behavior,
production-resource behavior, all fluid families, TConstruct startup/model
registration, multiplayer or flow/contact physics. The earlier native proof
and the new profile evidence must remain separately identified.
