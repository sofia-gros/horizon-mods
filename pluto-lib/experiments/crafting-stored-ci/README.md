# Stored crafting facts controller (source only)

This excluded controller connects the existing real API153 and finite policy16
preparer to the reviewed bounded Runner. Its one push-only workflow targets
**checkpoint/plutolib-crafting-stored-ci-20261106** only. Publishing these files
on the integration branch does not start this workflow. No trigger ref has been
created. Only a later coordinator release permits that action. Ordinary manifests,
the old four hooks, and bootstrap/pair/station candidates remain unchanged.

## Scope and sequence

- Current source base: integration 17f44dc36fc2df28547328aa8554286ae965b208.
- Official native input: 41f822b659789780f58d32c63ddfb6887ea1b8bf; its exact lock.
- Public repository/dedicated push event and checked-out head must match.
  Main, integration, PR and manual-dispatch events are rejected. The workflow
  uses the public standard runner, read-only token and no persistent checkout
  credentials; no other workflow is edited by this five-path source group.
- Source checks: controller31, preparer21, candidate13. No local native run.
- Cold official dependency fetch is locked. Subsequent metadata adaptation and
  all compilation are offline. Only the two local workspace packages may be
  added to the official lock, under the existing preparer's exact owner checks.
- One workspace, one real inventory API path, two complete test harnesses:
  API153 (147 unchanged plus six new) then finite16. Actual API test compilation,
  actual API library compilation for the fixture, four actual official native
  crate compilations and the finite harness compilation are separately checked.
- Each executable is identified from fresh Cargo JSON plus the actual rustc
  source witness. Both are compiled before listing/running either one. Existing
  generated native output trees may not change during the second compile; new
  output trees are allowed and the combined set is then sealed.
- No Cargo follows compilation. The two sealed binaries each list their entire
  inventory and run once, with no filter/ignored/measured tests. Separate
  api153/finite16 receipts are emitted; API147/6 subsets are separately named.

## Safety and evidence

The reused runner guards each writable filesystem with 10 GiB + 128 MiB and
requires a further 2 GiB before setup. Each phase has a finite timeout, one new
process group, TERM/KILL plus bounded reap and group-absence verification even
when the group leader already exited. Preparer Git/metadata children inherit
that group. Synthetic Python tests do not prove real descendant teardown.

Full source seals include regular-file bytes/modes, directory presence, and
symlink targets, including ignored/untracked entries; directory permission bits
are not sealed. Git must be clean before and after. Official source/lock,
workspace source/lock, binaries and all generated OUT_DIR trees are verified on
success and on failure as far as those objects were prepared. The raw phase log
has a 64 MiB bound, monitored during execution and checked before a bounded final
read; normal console emits only required names/summaries/compiler digests/receipts.
Raw logs remain ephemeral CI files, not retrievable from their hashes alone.

No Mixins plan/apply, actual constructor/bootstrap (five tests), installed Game
adapter typing, registration startup, output/remainders, station placement,
network/actor authorization or complete combined-game success is proved here.
Passing these finite checks cannot authorize removal of the old hooks by itself.
All Rust, metadata and CI execution remain UNRUN at source freeze.

## Cargo output classification

Compile stdout and stderr are separate bounded raw streams. A supplemental
locked/offline metadata read must reproduce the preparer's exact package IDs,
names, sources and paths; its unique actual name/version pairs with a custom-build target define the only accepted
non-JSON stdout prefixes (`[name version] `). JSON must parse without duplicate fields or non-JSON numeric constants, use a known Cargo
reason and a resolved package identity, with exactly one successful build-finished.
Unknown text or malformed JSON fails with a bounded line diagnostic; it is not
silently skipped. Fresh artifact selection additionally checks the exact package
ID, source, profile, executable kind/path and uniqueness. Actual rustc witnesses
come from stderr. This hardening follows the Guide compile-output failure without
asserting the unrecovered original stdout's cause.

Compiler witness parsing removes only explicit leading environment assignments, locates one actual rustc executable, and inspects only its following arguments. Homepage URLs ending in .rs and RUSTC environment values are not source/command witnesses. Unknown wrappers or ambiguous commands fail.

## Dedicated publication delta

The prior controller 95e35baf remains frozen at integration 73af9026. This source
group changes only the branch guard, pins the new workflow in the 52-file input
closure, extends the existing event/source tests without changing their count,
and adds that workflow. Runner lifecycle, Cargo JSON/actual-rustc parsers, native
inputs, all 153/16 test names, original policy bodies and compiler commands are
unchanged. The integration source cutoff remains provenance 17f44dc3; the actual
executing head must separately equal GITHUB_SHA and its whole checkout is sealed.
The dedicated ref must be checked absent before its single authorized creation.

The proposed workflow is not a successful CI run. Source-only checks remain
31 runner / 21 preparer / 13 candidate; native execution is still UNRUN.

Preparation note: the first local pure run passed 30/31; its previous no-workflow
assertion correctly rejected the newly authorized workflow. That assertion was
replaced with exact-one-pinned-workflow and no-trigger-creation checks, not removed.
This was a source test mismatch, not a Cargo, native, or CI failure.
