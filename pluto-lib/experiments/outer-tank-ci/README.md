# Guarded outer-tank CI source proposal

**Source-only next cutoff. Not published to its trigger branch or executed.**
The new fixed branch is `checkpoint/plutolib-outer-tank-native-ci-20261106`.
Its entry path checks the exact repository/pins and real-root memory admission
before setup; every command retains the same continuous guard. There is no
fallback on unknown topology. It does not create a cgroup or apply a hard cap.

The earlier [diagnostic run](https://github.com/sofia-gros/horizon-mods/actions/runs/42183008828)
at `9db4f9a2` intentionally stopped before setup. The separate `527bd9cc`
root-guard source checkpoint remained diagnostic-only. Neither ran native tests.
The 160-case suite remains unexecuted; its earlier 146-case predecessor and
pure author-API CI are separate proofs. Native trigger publication and execution
still require independent final review and the coordinator's remote slot.

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

## Root evidence and the next prepared admission

The [root analysis](../../docs/outer-tank-memory-root-20261106.md) records why
Linux's actual-parent file rule distinguishes the true root from a namespaced
non-root. The `527bd9cc` checkpoint passed 26 pure Python tests while retaining
the diagnostic-only entry. The historical diagnostic observation lacks some
new device/overmount checks and cannot be reused as a successful admission.

This next proposal has 29 pure Python checks: the same root regressions plus
pre-setup admission failure/no fallback, exact official/MOD source allowlists,
and whole-checkout ignored-file/link seals. Only one call owns the native driver,
after the memory check; the new mode and branch are fixed. The legacy diagnostic
helper still always stops and is not an execution fallback. Setup/native flags
remain false until their respective first operation. Missing required gates
also reject at final receipt assembly.

The source and official checkouts are sealed over every non-Git file and link,
including ignored/untracked additions, not just tracked bytes. Actual fixture
files remain regular-only and their complete old oracles are unchanged.
No Rust, Cargo metadata, cgroup/systemd write or local native job was run for
this proposal. Initial publication must be to a non-trigger source checkpoint;
CI admission is still a separate decision, not a claim that hard memory limits
have been installed.
