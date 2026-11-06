# Isolated Guide storage: actual Game typing result

The first dedicated CI run passed at exact source
`751999eaab8d7cd3e61312c698b134157feb7bfb`, tree
`bb449c4d2ed1368454eddeab6e6f8871ad738592`:
[run 42194400025](https://github.com/sofia-gros/horizon-mods/actions/runs/42194400025),
job 127813919427. The [machine-readable result](guide-storage-type-ci-result-20261106.json)
includes the complete retrieved compact console and final receipt. This closure
changes documentation only; it does not alter the tested prototype or CI inputs.

## What actually passed

- Fifteen runner parser/scope/failure checks and seven original prototype
  source/hunk checks against official Horizon
  `624c4f6265c02296fbe8a80f029dd0d987fb94c0`.
- The actual frozen proposed patch, originally published at `b434e9a4`, was
  regenerated in memory and applied only to a fresh CI-owned experiment copy.
  Exactly ten existing official files plus new `game/retained_book.rs` changed.
  The native BookScreen enum, Cargo manifests and Cargo.lock stayed unchanged.
- Full locked/offline metadata for pristine and experiment matched after
  normalizing only their source-root paths. Local package and target paths were
  constrained to their own tree; dependencies/features were not omitted.
- Actual compiler invocations for `horizon_client_lib` (`client_lib.rs`),
  `horizon_client` gameplay `src/lib.rs`, and its `src/main.rs` binary completed.
  The real Game field and BookScreen callers were therefore typechecked together,
  without a mock/extracted Game or server fixture. The check used no default
  features, debug/incremental disabled, one job, and locked/offline dependencies.
- Final Mod, official checkout, pristine export, experiment and build-script
  output seals passed. Minimum monitored free space was 89,471,102,976 bytes,
  above the 10 GiB + 128 MiB floor. No local native build or fork access occurred.

Metadata stdout/stderr were kept in separate files and the complete stdout was
parsed as JSON. No mixed-output JSON-line heuristic was used. The raw verbose
compiler log was checked on the ephemeral runner and hash-bound, not published.
The preserved console contains each phase terminal, log hashes and the final
receipt's required compiler identities/invocation hashes. Hashes alone cannot
recover the verbose raw log, and this result does not claim otherwise.

## What remains unproved

This is an **excluded proposed official host extension**, not a newly available
Mixins-only capability or a native package installation. No official repository
was edited. Ordinary Pluto manifests, production APIs and TConstruct LAN behavior
are unchanged. A later composition with ordinary Pluto and consumers must keep
its own exact drift and complete-plan guards; this result does not relax them.

The observed storage stamp is still not an owner handle. Native owned-open
acquisition, final opening/localization revalidation and a conditional retained
reply update remain missing. Actual Client/window behavior for overlays, identical
reopen, player/connection changes, restoration, late/duplicate replies and
destructor outcomes was not executed. Neither language-change page rebuilding nor
the legacy maximum reply envelope/decoder connection was tested or activated.

Separate evidence stays separate: Guide identity10 proves pure consumer helpers;
Font12 proves its bounded native Font/Text CPU contract. The common bundle result
at `0468d4b1` has a complete retrieved receipt for the same installed language8 and
Key6 sources ([closure ae8d4f6b](https://github.com/sofia-gros/horizon-mods/commit/ae8d4f6bbcc56087e9b6276e8e0265d3b61d5ee6)).
That later result does not retroactively recover the earlier `0fb35fec` job's
missing final log, and neither language result proves a retained Guide refresh.

Next is a separately reviewed ownership/update boundary and ordinary-package
composition experiment, followed by real Client/LAN acceptance. No unsafe screen
mutation or old-feature disablement is enabled merely because typing succeeded.
