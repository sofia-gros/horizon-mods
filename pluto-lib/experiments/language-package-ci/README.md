# WIP actual language-package CI

Prepared after successful native i18n22 run `42181415662` at `0e4d68ea`. This
separate runner is **not yet published to its trigger branch or executed**.
It adds no production module, resource pipeline, translation dictionary, locale
setting, consumer code or installation step. All eight Rust fixture cases are
still uncompiled and unrun.

## Fixed source and scope

Base: `09321be86bae00951c421cbda2469417635c4a75` on the language checkpoint.
Fixture: exact `4d776b907aff5e748c700dafa422d791ecaf99ac` six-file source group;
`language-package/fixture.rs` and `Cargo.toml.in` remain byte-identical. The
Package→Content→Language eight cases are separate from the already-passed
internal8/Key6/item8 suite. No result or count is borrowed from i18n22.

The only dependencies are the actual official Horizon `624c4f62` client-core
and horizon-mods crates (no optional index feature requested). The fresh CI
checkout is sealed; the official lock is seeded and resolved offline after a
locked official fetch. Metadata must point every local dependency to the exact
official crate path and the sole fixture to its own path. Unknown dependency
identities, aliases, duplicate local names and changed source bytes reject.

The actual compiler log must contain cold rustc invocations for
`horizon_client_core`, `horizon_mods`, and `pluto_language_package_fixture --test`.
Then the same target runs all eight exact names once, with zero ignored/filtered
cases. Any recompilation found in the completed execution log fails the proof;
this is post-run detection, not interception before rustc starts. Generated OUT_DIRs are sealed between compile and
execution. Both complete checkouts, fixture/lock/metadata file set and generated
outputs are reverified on success or failure. Full checkout seals include
untracked/ignored files and symlink targets; source status also rejects ignored
files. The official tracked documentation symlink is retained, not followed into
a copied source tree.

## Safety and resource limits

Only the named public target repository branch can trigger this standard
ubuntu-24.04 job. It uses contents:read, the pinned existing checkout action,
no persisted credentials, no secrets, no cache/action artifacts, no deployment,
no external communication and no fork source. The official source origin and
immutable revision and lock are allowlisted before any network request. Local execution is not a fallback.

The inherited successful runner uses one build job, debug0/incremental0, cold
Cargo/target directories, 10 GiB + 128 MiB continuous free-space floor across all
write filesystems, and a 2 GiB setup reserve. Timeout/disk/failure stops only its
own process group and verifies that the group is gone. Every attempt prints a
partial FINAL_RECEIPT; passed requires all named gates both inside execution and at receipt finalization, plus successful final
verification. Production source and existing CI inputs never change.

## Source validation and review status

Twenty Python runner regressions passed locally in 0.017 seconds; five existing
fixture source guards had passed separately against exact official files. One
initial adapted runner test looked for the old i18n-only scope sentence; that
assertion was corrected to the new explicit Game-admission exclusion, and the
complete suite was rerun. No Rust, Cargo, metadata, Mixins plan/apply or host
build ran locally. Independent review is required before publishing the trigger
branch and running the job.

The fixture uses a test-only ResourceSource view over admitted content bytes.
It does not run Game::content_root enabled/dev admission, native pack/content
overlay, cache rebuild, Options language changes, rendering, LAN or live UI.
A source review of the eight native cases found no concrete serious defect;
that review is not their execution. Runtime TOML remains unsupported: optional
Python TOML preparation produces JSON for the consumer's own content package.
