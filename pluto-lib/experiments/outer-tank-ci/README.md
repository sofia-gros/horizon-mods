# Outer-tank diagnostic-only CI preparation

**The first CI is diagnostic-only. No setup, toolchain installation, additional
Git fetch, Cargo, compilation or tests are reachable from its entry point.
It deliberately produces a false receipt after read-only memory observations.
No CI publication or native execution has occurred yet.** The existing 160-case fixture remains unexecuted. Its
prior 146-case predecessor and pure author-API CI are separate proofs.

This dedicated public branch is intended to close the local disk-budget blocker
without borrowing its cache or changing any source under test. Publication and
execution await independent review and the coordinator's remote slot.

## Exact inputs and scope

`inputs.json` records the same 55 selected blobs from remote source
`794a0c526ca0a96d0ca9c9ff40010611b5968a89`, tree
`0e80347d4573f47984753cd02f78514a5352243f`. These include the complete historical
28-file actual inventory API and all frozen old methods. None is replaced by the
new authoring facade or the current working tree. The old whole-file restoration
guards must still prove that only the three intended outer `tank` edits changed.

The prepared fixture uses official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`, its exact Cargo.lock, actual
`horizon-items`, `horizon-world`, `horizon-client-core` and `horizon-core`. It does
not compile `horizon-client`, renderer or Mixins, and does not apply a Mod.

The complete existing Cannon source is a byte-exact prefix. The three complete
old methods are still the differential oracles. Controller and Drain route
bodies are unchanged from the frozen harness. **Drain uses its existing mock
Game/world.** Comparing its saved proposals and counters is not proof of native
actor/slot authority, full gameplay, disk durability or rollback.

Expected result: one full suite of exactly 160 tests, 0 ignored/filtered, with
all four named new cases required. The 156 imported tests are distinguished from
the four additions: Cannon 128 matrix comparisons, four retained-wire variants
plus opaque item preservation, 24 controller comparisons and 576 Drain route
comparisons. These are expectations, not results.

## Preparing without reimplementing the fixture

The original `run_outer_tank_patch.py` is hash-bound. Its preparation prefix ends
immediately before the first Cargo call. Two explicit textual substitutions
relocate only the frozen source root and temporary output directory. Inverting
them must recover the complete original prefix byte-for-byte. The prefix is
parsed to reject subprocess calls before execution; its real dependency-helper
import remains unchanged.

Preparation runs twice into separate fresh directories. Generated Rust and
Cargo.toml must match after replacing only the second output-directory path.
The Cannon prefix plus renamed complete oracle is independently reconstructed
and compared. No production Rust, native declaration or fixture behavior is
reimplemented. Generated files and the complete 55-file source set are sealed.

## Cold CI and failure rules

- A new dedicated GitHub-hosted job, no persisted credentials, secrets, cache
  restore, artifact upload or PR triggers. Exact public branch and HEAD only.
- All fetching/setup/commands run under continuous disk and actual RAM/cgroup
  guards. Disk floor is 10 GiB + 128 MiB on every output filesystem, plus a
  4 GiB reserve before setup/cold phases. Measured effective RAM must be at
  least 6 GiB, at least 2 GiB available at admission, and 512 MiB during commands.
  These are fail-closed admission/stop thresholds, not a claimed peak bound.
- Official Git source and toolchain are fetched fresh. Cold fixture metadata
  can prune the official lock and add only the two declared local packages.
  Every registry identity and all six native/API/fixture package paths must
  match before compilation. Fetch is then locked; compilation/tests are offline.
- Jobs 1, debug 0, incremental 0, no local shared target. A real cold compile
  must show all four native crates, actual inventory API and the fixture test
  crate. `Fresh` alone cannot satisfy this requirement.
- After compilation, a rustc gate refuses every further compilation before it
  executes. Only bounded capability queries remain permitted. The full test
  suite runs once; exact count, names, no skips and generated OUT_DIR stability
  are mandatory. No automatic retry after errors, guard stops or source changes.
- Final full source/file-set/lock/official seals and clean/untracked checks run
  even after failure. Own process groups are terminated and checked, including
  descendants whose leader already exited. A final receipt never labels an
  incomplete gate as passed.

This proof is a separate historical runtime cutoff. It does not upgrade any
newer source, install modules or prove arbitrary lossless persistence. In
particular the local 32 MiB cached-run estimate is not reused as a cold-build
budget, and no dependency is assumed Fresh from matching paths or lock alone.

## Source-only preparation result

Seventeen pure Python guard regressions passed. The unchanged 55 inputs were
materialized from their recorded local original commits with exact published
hash/length checks. Both relocations inverted exactly; two independent native
fixture preparations produced the same three files after only their output-path
relocation. No Cargo, metadata or Rust process was run. The remote commit object
was not present locally, so the initial read-only remote-SHA git-show check
stopped; CI still must fetch and verify the actual remote tree. See
`preparation.json`. These checks do not turn the expected 160 Rust tests into
executed tests.

A read-only `MEMORY_PREFLIGHT` record now captures bounded host-memory values,
self cgroup membership, visible cgroup2 mount root/point and each visible
ancestor's memory/controller/events/type fields. Missing facts remain explicit
errors. This is diagnostic evidence, not a guard exception: a true hierarchy
root may lack memory.max/current, but hidden/delegated ancestors cannot be
assumed unlimited. The first CI attempt must stop before native setup
with a false receipt and these observations. The initial diagnostic checkpoint contained no root exception. The prepared
source-only guard below still needs separate review before native admission.

The native driver below this entry point is retained draft source only; it has
no call site and cannot be selected by CLI/config/environment. A readable
delegated cgroup root can hide stricter ancestors even when its visible
memory.max says max, so the old visible-limit loop is not sufficient approval.
Actual topology observations and a separately reviewed new cutoff are required
before enabling any native path. Expected 160-case instructions above describe
that later proposed proof, not this diagnostic execution.

## Prepared real-root guard, still diagnostic-only

The [source-only root analysis](../../docs/outer-tank-memory-root-20261106.md)
binds the narrow missing-root-files exception to Linux's actual-parent file
creation rule. Its 26 pure Python regressions pass, including namespace roots,
permission failures, mount/device masking, symlinks and ancestor limits. It
does not enable the native draft or apply any cgroup/systemd setting. The
earlier diagnostic observation is not retroactively sufficient for its new
mount/device checks. A separate reviewed execution decision is still required.
