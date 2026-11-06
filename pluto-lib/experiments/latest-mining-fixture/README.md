# Mining scalar fixture preparation

Source-only until the dedicated CI executes. This prepares 24 unchanged tests
from main08afa mining rules/math plus the frozen 12 additional scalar cases at
5d343c17. It uses actual f4dc horizon-items/core/world dependencies and native Tag,
not a substitute Game, Items implementation or synthetic component API.

The four old policy/tool source files are read from the exact MOD Git commit,
with path and SHA-256 checked by the frozen renderer. The official game/hand/LAN
files are also byte-pinned. The output is seven unchanged/rendered Rust files,
one local Cargo manifest, an adapted subset of the official lock, and a receipt.
A no-checkout Git object store is only an immutable source oracle, not a runtime.

The full binary must contain all 36 names. The 24 existing cases and 12 extra
cases are separate disjoint receipt groups but are executed together once,
without filters. Rust and real fixture preparation are initially UNRUN.

This does not compile the new Game adapter, shared apply, authoritative actor
selection or network code. Official numeric comparisons are source-pinned
predicate projections, not an assertion that complete host and guest paths are
equivalent. In particular 0/0 is NaN and the native delayed-guest comparison can
choose a different branch from the local threshold. The adapter adds no clamp;
unchanged TConstruct conditional math still ends with its existing max(0.0).

The preparer is only executed as the dedicated controller's process-group leader.
Git and metadata inherit that group. A timeout kills/reaps the immediate child;
the controller always bounds teardown of the complete group, including if this
preparer is killed before its finally block. The tests here mock that contract,
not a demonstrated OS descendant teardown. All source file sets, regular modes,
links, locks and generated files are checked before and after the real proof.
Cold registry dependencies are seeded from the exact official --locked fetch;
fixture lock adaptation is offline and accepts only official dependency identities
plus the one local harness. Builds are locked/offline thereafter.
