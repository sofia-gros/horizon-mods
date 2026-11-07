# Isolated guide lifetime experiment

This directory is **not installed by the PlutoLib native package**. It is an
experimental candidate at official Horizon 624c4f62 / Mixins e8cd0aa3, separate
from the frozen, validated current-book service. Do not activate it as a completed
guide session API.

It contains an owned ephemeral State, a checked process-wide opening allocator,
a checked latest-request sequence and consumption before any later effect.
Nine real native invalidation HEADs and one explicit owned ChunkMap field are
proposed. The native adapter reuses the existing current-book context/Text
projection and adds current Remote level/epoch validation. There is deliberately
**no native activation, network subscriber/send, wire decoder or screen update**.
The pure State::open method is exercised only by state tests; a native matching
book never activates it. No TConstruct content or rules appear here.

## Executable evidence

- `rustc --edition 2024 --test mods/pluto_guide_lifetime_experiment/state.rs -o /tmp/pluto-guide-lifetime-state-tests`
- `/tmp/pluto-guide-lifetime-state-tests --test-threads=1`
- `python3 test_guard.py`

The 11 state tests pass for explicit invalidation, same-epoch player reset,
nonreused opening identity, dropped hidden replies, duplicate/out-of-order
requests, owner/channel mismatch, map replacement, two independent state owners
and nonce/request exhaustion. These are **pure state tests**: named lifecycle
events call invalidate in the test. They do not execute a real client/window,
actual native open/close callbacks, native respawn or network delivery. The
12 guard tests exercise reviewed/no-op plan contracts and adversarial changes;
they are not an actual applied-host result.

Use only a complete actual plan including the frozen PlutoLib current-book
service. Run its common mandatory guards plus:

`python3 check_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE`

before any experimental apply. The guard pins source/helper/module/state bytes
and constructor identity. The nine mutation boundaries are exclusive except
for the three exact reviewed, disjoint PlutoLib ChunkMap clear helpers. Other
aliases, replacement/cancellation/interior hooks fail closed. This is not a
sandbox for arbitrary trusted injected code. Raw unguarded application is not
supported. No native body is copied.

Actual complete plan, central nine guards plus this experiment guard, apply and
client lib/bin typecheck passed for library `529ad722` plus experiment `1d3eb02f`
on the official 624c/e8cd pair. The check took 66.06 seconds; 5,041 generated
files were sealed, and minimum free disk was 11,187,482,624 bytes with no disk
stop. This is a separate **library-plus-experiment type proof**, not an installed
library release or evidence that the hooks ran in a live client. See
`validation.json` for exact cutoffs, commands, hashes and retained rejections.
The earlier successful no-op primitive-state probe is not substituted for it.
No official Client+renderer/window fixture or complete approved prepared resource
tree is available. Official remote.rs has an ignored Client-role fixture using
silent audio and Pipe::pair, but it needs HORIZON_TEST_RESOURCES and has no
renderer. Do not relabel an IntegratedServer fixture or weaken the presentation
gate to make a positive path pass. A real live opening/reopening/response test
therefore remains a blocker even if typechecking succeeds.

The experiment does not change the established packet-observer ordering; there
is no reply subscription here. A future effectful adapter still needs its own
atomic activation, bounded envelope, actual current-book operation, lifecycle
and duplicate-effect tests. Read the full audit in
`../../docs/guide-lifetime-feasibility.md` before expanding this experiment.

The first richer actual plan rejected inner file documentation attributes with
`E_MODULE`. The experiment changed `//!` headers to ordinary `//` comments; no
state logic or native effect changed. That rejected attempt is retained separately.

The corrected plan and central library guards passed; the experiment guard then
rejected the newly included saved-book clear helper before apply. Its exact
disjoint State::clear was reviewed, and the three admitted clear-state modules
are now also pinned byte-for-byte. The rejected attempt remains retained.
