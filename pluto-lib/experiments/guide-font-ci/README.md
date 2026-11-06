# Dedicated Guide Font12 checkpoint CI

This WIP workflow runs only on push to
`checkpoint/plutolib-guide-font-ci-20261106`. It does not change the default
workflow, old manual terrain CPU/Game branches, PRs, permissions or billing.
It uses a standard public-repository Ubuntu runner with a read-only token, no
persisted credentials, artifact upload, cache action or external asset download.

The exact ordinary v15 base is local `8715b123` / remote `89744814`.
Only the ten Guide source/document paths from local `59c0651b` / remote
`c9bc56ee` are added, alongside this runner and its dedicated workflow.
`inputs.json` pins their bytes, complete shared API file set, native preparers,
official `horizon-rust/horizon` revision `624c4f6265c02296fbe8a80f029dd0d987fb94c0`
and Cargo.lock. It does not use a fork or apply Mixins.

Cold-runner preparation first fetches the official locked dependencies. All
fixture metadata, compilation and execution then use offline locked identities.
The real native core/render and fixture rustc invocations must be observed.
The twelve exact `consumer_tests` names must all pass with none ignored and
eleven other helper tests filtered out. Zero tests cannot pass this gate.
Source/file-set/Cargo.lock seals cover the original MOD, official source and
prepared fixture before and after Cargo. Generated native OUT_DIR files are
sealed after compilation and compared after execution.

Compiler jobs are one, debug and incremental outputs disabled. All outputs
are runner-local outside official and fixture source trees. A 10 GiB +128 MiB
free-space floor is sampled throughout each child process; only that child's
process group is stopped on failure. Polling is a bounded monitor, not a hard
filesystem quota. No local native build is authorized by this source change.

Before dispatch/publication: six small parser/scope checks passed. The actual
Font12 result is **not run** until an exact matching Actions run completes.
This fixture covers actual Font/Text and an original tiny CPU atlas resource.
It is not Game compilation, Mixins apply, live input, full book drawing,
GPU display, gameplay or installed consumer validation. CPU terrain and Game
borrow checks remain separate, unchanged, unexecuted proof jobs.
