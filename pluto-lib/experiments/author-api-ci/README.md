# Mod-author API layers: exact pure-crate validation

**Prepared, not yet run. This is a validation-only composition, not an installed
native package or a main-ready TConstruct migration.**

The user's direction is two logical layers: maintainable typed internal services
and a small Fabric-like API for mod authors. These are not two new Rust crates.
The existing two bundled crates keep their current host dependency owners.
Public wrappers delegate to the same implementation and registry; the old
functions, structs and host-qualified paths are not removed. Horizon/Mixins
adaptation remains inside the library. This does not promise Java syntax,
Fabric ABI compatibility or compatibility with untested future host revisions.

## Exact combined inputs

The base is `2f692e4530acbfeef4f5b21be7708f8766e514ed`, the separately proved
134-test recipe-presentation API atop the Java/menu foundation. This branch
adds only the selected pure API and author-example paths, plus this dedicated
CI. Inputs and source sets are recorded in `inputs.json`.

- Guide `39499dc85ab4489b251a0cf0216bb37299f667ae`: zero-sized payload event
  entry, borrowed channel configuration, guide type re-exports and Span
  constructors. Its internal registry/observer/codec remain the same.
- Recipe `dd491b3a1e6d0431eeac6965c2d18ed9f0a1ea11`: owned Replacement
  constructors/builders and two public API examples. Native fixture changes
  from that source checkpoint are deliberately excluded.
- Fluid `bf9f64e7d179f06af34bf781c01dac7afe212c82`: the complete real public
  API at that source, including paired Declaration/render facts. Its native
  render adapters are NOT imported here. The old fluid author example is from
  `3c778609e2660a44774f2504964abc02164b7d0f`.
- Storage example `0e54bcb24571545f3bdde5bac7d1807fe60cbbd7`: required plan
  success and six assertions over caller-owned values; no native slot commit.

The resulting inventory API has 32 exact files including its Guide example.
The public API has 11. The source parser counts 143 and 58 `#[test]` cases
respectively; the CI must execute those complete library suites without a
filter and verify every passing result, with zero ignored or filtered tests.
The nine new Guide/Recipe and six Declaration test names are explicitly required.
No mock API crate, copied Game/ItemStack or stubbed host adapter is used.

## Separate evidence gates

1. Thirteen source/scope/parser/mocked runner checks. They do not run Rust.
2. Real `pluto-inventory-api`: 143 library tests, including legacy and wrapper
   use of the same registration/validation paths.
3. Real `pluto-api`: 58 library tests, including the six Declaration cases.
4. Guide's actual standalone example: one test invokes its explicit startup,
   constructs a document, and explicitly removes its subscription.
5. An independent consumer crate imports both real API packages. Cargo metadata
   must show exactly these two local packages plus the consumer, at their
   expected paths. The two Recipe binaries and Storage binary are type checked;
   each needs an observed actual rustc invocation. Recipe main functions are
   empty, so they are **not** claimed as runtime behavior tests.
6. Storage is built and its exact binary is run. Its six assertions must be
   reached; the source no longer permits an empty successful planning path.
7. One small consumer test calls the unchanged fluid author example and checks
   idempotent registration, dry/unknown/registered-source contact results using
   actual public API types. Its supplied read closure is not a native world,
   actor capability or contact event.

Each Rust test target is compiled first. The subsequent execution rejects
unexpected recompilation and checks generated OUT_DIR inputs did not change.
A fresh invocation for each designated library/test/bin must be visible; a
Cargo Fresh-only success is insufficient at its compile gate. Complete source,
file-set, manifest and lock seals are checked before/after, and the checkout
must remain clean including untracked files.

## Runner and dependency safety

This workflow runs only on a push to the exact dedicated public repository
branch `checkpoint/plutolib-author-api-ci-20261106`. It uses a standard Ubuntu
runner, contents-read only, pinned checkout with no persisted credentials,
no artifact/cache upload and no PR, repository-permission or billing action.
Same-branch supersession cancels only this workflow's own group. Existing
manual CPU/Game, Font and fluid-render workflows are not changed or selected.

The official dependency reference is `horizon-rust/horizon` at
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`; only its exact Cargo.lock is used for
registry identity validation. This is not an official source/host compilation.
Cold API fetches are locked; all compilation/execution is then locked/offline.
Registry package names, versions, source URLs and checksums must match that
lock. The external consumer lock adds only its root and the real public API to
the existing inventory seed. No Cargo metadata/path match is described as proof
of cache freshness: the target is new and compiler evidence is inspected.

All setup, subprocesses and outputs are guarded across checkout, work, Cargo,
root and actual Rustup filesystems. Require 10 GiB + 128 MiB throughout and an
additional 2 GiB before setup. These are polling limits, not a quota. Jobs1,
debug0, incremental0, isolated Cargo/target/TMPDIR; bounded TERM/KILL/reap and
own-process-group absence are required even after a normal leader exit.
No local native job is part of preparing this workflow.

## Non-claims and compatibility boundaries

The CI validates this exact generic API composition and examples. It does not
apply the mixed source tree to Horizon, validate native Book/display adapters,
run the Font/GPU/HUD, establish LAN or retained-book reply ownership, dispatch
immediate effects, or activate any TConstruct feature. The older Font12 proof
stays at its own immutable cutoff. New i18n and later facade work are excluded.

Callbacks still have their original phase, order, bounds, error and lifetime
rules. Observation supplies no send/cancel/identity/actor authority. A fluent
proposal supplies no placement/freshness authority. A Declaration's paired
registration does not install assets or prove an actual rendered fluid. New
source-compatible entry points are tested against existing calls; untested
host versions and live display paths remain explicitly unverified.

Preparation: 13 Python checks pass. The first local attempt imported the runner
from an unrealistically shallow temporary path and stopped before tests; it was
rerun from its exact repository-shaped path. No Rust or CI result is claimed
until an exact matching run reaches terminal status. Evidence is retained in
ordinary CI logs/summary and a later docs-only closure.
