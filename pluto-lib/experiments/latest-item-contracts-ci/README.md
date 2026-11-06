# WIP latest native item contracts CI

Source preparation only: no workflow trigger, real preparation, metadata or Rust
execution has occurred for this group. The dedicated push-only public standard
runner uses a read-only token, no caches, uploaded artifacts, secrets, billing
changes, PR trigger or dispatch expansion. Publish to the non-trigger source
branch first; independent review and explicit serial release precede execution.

The exact official target is Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`.
This group does not use Mixins or change any normal manifest. Old item-name32
success on `624c` is separate; latest removal synchronization, default-name
ownership/population, installed Game display and package coexistence are not
proved here. The old two fixture literal changes remain unapplied.

## Independent fixtures, one cold dependency graph

- Template6: original six tests plus the byte-exact latest native Template through
  json_tag fragment. Actual items/world/core dependencies, no mocked Template.
- Default4: the unchanged four historical catalog/Weak-cache tests against latest
  actual native crates. These document existing behavior, not a fixed name owner.

Each has its own real Cargo package, copied/adapted official lock, source receipt,
actual test compiler invocation, distinct binary, exact complete name inventory,
one direct binary execution and per-fixture receipt. The first compile must show
actual native core/client-core/items/world compiler invocations with exact source
paths. The second may reuse that same graph, without claiming another native
compilation. Both harnesses compile before either test binary executes. Shared
OUT_DIR sources must remain unchanged between builds and through test execution.
No Cargo command runs during the test-execution phase. The expected summaries are
6/6 and 4/4, with zero failures, ignored or filtered tests, not an undifferentiated
10-test claim. The source file comments remain historical preparation labels.

## Evidence and boundaries

Inputs freeze complete source groups and both preparers. Official Git checkout
identity, exact lock, tracked/untracked/ignored cleanliness, regular-file bytes and
modes, directory kinds and link targets are checked before and after. No link is
followed by a source seal. Directories' permission bits are not sealed. Generated
OUT_DIR links are refused, and both executable byte hashes are checked before and
after execution. Offline metadata may adapt only the copied fixture locks to the
real-host graph; all final compilation is locked/offline. Cold dependency fetch
uses the pinned official lock before offline mode. Both preparers require the dedicated controller marker and own PGID-leader
identity; metadata/Git inherit that outer group rather than creating a detached
session. Internal timeout kills/reaps the direct child and fails, while outer
teardown always removes remaining descendants, including when the preparer is
SIGKILLed. All outer child groups have bounded teardown; all writable filesystems keep 10 GiB + 128 MiB free and a 2 GiB startup
reserve. Failures do not auto-retry or enable production hooks.

All raw logs are inspected locally on the runner, while the console prints only
small compiler witnesses, test names, phase hashes and final/per-fixture receipts.
Raw phase files are ephemeral; a hash does not make them recoverable later. A
failed gate remains failed even if later source verification succeeds.

The initial local preparation of the latest default helper stopped because the
immutable latest loot.rs hash input had not yet been fetched; three guard checks
therefore errored on missing inputs.json. Fetching that official source completed
the input ledger without changing Rust oracles. The final 15 preparer guards pass.
No real Cargo/metadata/native operation occurred during that preparation failure.

Independent review of the initial 8cd4 controller found a nested-session escape
risk in both preparers. This source-only correction removes those nested PGIDs
and adds three negative/lifecycle checks per preparer plus a controller wiring
guard. Rust6/4 bodies and production runtime remain unchanged. No run was started
with the earlier runner. Final counts are 24 runner, 20 Template preparer and
18 default preparer Python guards; these remain separate from native test results.
