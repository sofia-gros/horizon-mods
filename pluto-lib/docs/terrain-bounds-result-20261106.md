# Terrain private-Batch bounds: isolated result

[Run 42218495299](https://github.com/sofia-gros/horizon-mods/actions/runs/42218495299)
completed successfully on its first attempt at exact source
[`43e72e9e5a4d64c775fe172d4c0e678e6de74e63`](https://github.com/sofia-gros/horizon-mods/commit/43e72e9e5a4d64c775fe172d4c0e678e6de74e63)
(tree `a767b8c5be7e146c7f4e30a81c239331950cddd0`). GitHub created the run at
12:32:51 UTC and recorded terminal success at 12:33:14 UTC on the standard public runner.
Job ID: `127894381802`.

This is a **std-only predicate comparison**, not a production optimization or
an actual host/Mesher test. The original private `Batch` module, original
validation function and proposed smaller predicate were compiled into an
excluded fixture. Production, installed guards, old CPU oracle and profile
sources did not change.

## Verified outcome

- All 11 required gates passed; final `passed` and `verified` are true with no
  error. The 20 controller, 11 bounds-source and 8 host-composer Python checks
  passed before toolchain setup.
- Actual Rust 1.95.0 compiled two new test executables with explicit opt-level
  0 and 3, debug assertions and overflow checks enabled, debug information off,
  16 codegen units and LTO off. Each executable had exactly the expected 14
  tests, then passed all 14 in a separate process with no ignored/filtered tests.
- Compiler, generated source, executable and complete checkout seals remained
  equal. The final receipt records exact compiler argv, binary SHA/size,
  source seal and every phase's output hash/bytes. Post-run retrieval independently
  checked both named 14-test console results and the explicit compile flags.
- Both builds completed under the unchanged per-process 1 GiB virtual address
  limit, 120-second CPU limit and 128 MiB file-size limit. This does not measure
  total group RAM. Minimum observed free disk was **91,305,795,584 bytes**,
  above the 10 GiB + 128 MiB floor. No local compiler or official checkout ran.

The eight host-composer checks only establish the one-call source transform and
complete inverse back to the original host file. They do not type-check that
host. Embedded composer receipts retain their source-only flags because those
describe the composition phase; the later compile/test gates are the Rust proof.

## Timing limits

The fixture recorded seven paired totals for 4,096 predicate calls after 128
warmup calls, for each of four fixed cases in each profile. All raw arrays are
retained in the companion JSON; no unfavorable or short observations are removed.

Only opt0 `front6` and `paired12` met the predefined duration threshold for both
predicates. Opt0 `nonlocal`/`empty`, and **all four opt3 cases**, are explicitly
inconclusive because at least one sample was too short. Six of eight micro cases
therefore provide no performance conclusion. The observed clock interval was
21 ns. No iteration extension or automatic retry was performed.

Even the two non-short opt0 comparisons cover only these fixed pure predicates.
They cannot establish optimized shipping benefit, actual mesh cost, provider
callback cost, allocation savings or game frame time. Compiler specialization,
call overhead and the fixed inputs remain limitations. The successful older
whole-CPU mesh results are separate and are not relabeled as this experiment.

## Evidence retention and remaining gates

The exact decoded GitHub console retrieved through the normal job-log tool is
32,445 UTF-8 bytes including its initial BOM, SHA256
`31a72c7740fd0aba72ff1eb7bbf3aca961e0c7faadb51dedf7a3d48189a129cd`.
The companion [JSON receipt](terrain-bounds-result-20261106.json) preserves the
complete final receipt, raw phase hashes, test names and timing arrays. Raw
per-phase output was ephemeral inside the runner and was not uploaded. The
retrieved console is retained locally rather than committed wholesale.

Actual latest complete-plan acceptance, rendering identifiers/guards, host
typing, whole-Mesher CPU differential/replay/profile, real game assets,
TConstruct activation and GPU display remain separate, unproven gates here.
The production predicate is unchanged; this result alone does not authorize
adopting the proposed optimization.
