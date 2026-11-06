# Loaded advancement-tree projection, 2026-11-05

This is another slice of the broad advancement API group, not a new PR or a
completed TConstruct discovery migration. It is outside the earlier verified
complete-library cutoff 0dd927ef.

## Boundary

The shared API replaces only criteria, requirements and recipe rewards on
existing goals, using complete normalized expected definitions. All other native
fields are preserved. Consumer IDs, goal eligibility, material variants, source
shape checks and completed-goal behavior remain outside PlutoLib. There is no
extra registry/cache, automatic hook, actor-progress mutation or forced prefetch.

Inspection found the old consumer reconstruction indexed only recipes, while
current official Horizon also requires looking-at, NBT and player-fact indexes.
The shared adapter rebuilds all four plus the trigger index. Its snapshots also
include reward functions, which the older consumer shape comparison omitted.
This does not itself repair or activate that old consumer.

## Focused evidence

The fixture uses clean official Horizon
624c4f6265c02296fbe8a80f029dd0d987fb94c0. Complete official advancements.rs bytes
are the exact prefix of a dependency module with the production projection child
and a fixture-only native parser entry appended. Potion and NBT references use
the real official core modules. No parser/index/type implementation is stubbed.
The full real Pluto inventory API is a single Cargo path package. The outer Game
ownership shell is modeled explicitly; this is not a running game or LAN test.

The preparation/verification tool checks official origin, clean immutable head,
source prefix/hash, all copied helper/API hashes, exact local Cargo package paths
and resolved dependency identities against the official lock. Tests run offline
and locked, debug information and incremental compilation disabled, two jobs,
under a continuous 10 GiB + 128 MiB free-space guard.

Behavior coverage: native field/goal-position preservation; all derived indexes;
no-op Arc identity; atomic stale/missing/duplicate/invalid rejection; parser
normalization rejection; missing/inconsistent lookup; remote/unloaded rejection;
and no changes to modeled actor, progress, event hash or pending-tree markers.

The first source review caught inner documentation comments unsupported by the
Mixins module parser. Both native headers were corrected to ordinary comments,
and a packaging regression protects that constraint. This was a packaging issue,
not a Rust fixture failure. Final exact-source test results are recorded below.

The later complete native package gate is recorded below. Bounded fixture
success by itself is not that full gate and is not source-goal/runtime readiness.

## Final source results

- 7 exact-source native projection tests passed after the header correction.
- 26 shared API unit tests passed; all API bytes match the final fixture.
- 7 deterministic package/source guard tests passed.
- Final native fixture took 1.203 seconds. Minimum free space was
  11,030,933,504 bytes; the guard did not stop execution.
- Independent brief source review found no further serious atomicity or
  preservation issue after the header correction. It was not a lifecycle audit.

Evidence is under .work/official-advancement-recovery: projection-fixture-final/
fixture-sources.json, projection-final-test.log, projection-final-result.json,
projection-final-identity.log, projection-api-test.log and
projection-package-tests.log. The adjacent JSON report pins production source
hashes and the exact official source hash.

To reproduce, prepare a new output with tools/prepare_advancement_projection_fixture.py
using --horizon-source pointing to the clean pinned official checkout, then run
cargo test --offline --locked --manifest-path OUTPUT/Cargo.toml --lib in the
coordinated build slot. Run the preparation tool with --output OUTPUT --verify-lock
afterward. The helper never fetches or edits the official checkout.

## Complete native package gate

The actual complete PlutoLib package at
999468ef38bd542e2e4dc8f83205f26462ad6e33 includes these unchanged projection
sources. On the same official 624c4f62/e8cd0aa3 baseline, planning, mandatory native
preflight, apply and horizon-client library plus binary check all passed. The
package contains 2 crates, 34 modules, 25 hooks and 2 supported states. Archive
SHA256: 2737204f0ead5e3e57757532279904145d8a935ed8d28b296ac9823f4eeeea45.
The check exited 0 after 85 seconds with a minimum 11,244,822,528 free bytes and
no disk-guard stop. Warnings remain; this is not a warning-free build claim.

Evidence: the central integration worktree's
.work/plutolib-foundation-proof/grouped-v4/{cutoff,plan,result}.json,
native-preflight.log, apply.log and host-check.log. The shared foundation report
is the central index of this proof. No TConstruct bundle, automatic projection,
preload integration, game/LAN proof or original acquisition-stack certification
is included.
