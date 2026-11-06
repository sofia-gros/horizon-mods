# Language package distribution: eight-case result

**Passed, first CI attempt.** The exact reviewed source at
`8b17fba9610c3e60a873c822f2fa04f93d5c94e4` ran in
[run 42184962517](https://github.com/sofia-gros/horizon-mods/actions/runs/42184962517),
job `127783013103`. The result is recorded separately from the earlier i18n22
and installed-client-library language proofs.

## What ran

- Actual official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`
  `horizon-mods` and `horizon-client-core` crates, with the official seeded lock.
- Real Package writer/reader, Content admission and byte reads, then native
  Language loading through the explicit test-only ResourceSource view.
- Eight exact Rust cases passed in one complete suite: 0 failures, ignored,
  measured or filtered cases. The native test result took 0.02 seconds; the
  cold compile phase took 43.299 seconds.
- Twenty CI-runner regressions and five source guards also passed.

The eight cases cover owned English/Japanese assets surviving an actual .hmod
round trip, selected-language and English fallback, no guessed regional fallback,
namespace and registries-capability admission, TOML assets not becoming runtime
language support, malformed locale bytes remaining the native loader's concern,
custom namespaces not implying locale-menu discovery, and an admitted file's
later I/O failure being a real failure rather than silent missing-resource success.

## Identity and integrity

Frozen fixture source is `4d776b907aff5e748c700dafa422d791ecaf99ac`; the CI
source corresponds to local `1c8aa9e04e8b259345f6607e1adc6f8992732a14`.
The raw log contains actual rustc invocations for both official crates and the
fixture with --test, all eight exact names, and a single successful final receipt.
The receipt reports passed=true, verified=true, error=null. Before/after official
inputs, fixture source/lock/metadata, full checkout file sets and symlink targets,
and generated OUT_DIR hashes agree. Execution-log recompilation detection found
no recompilation. This does not claim permission-mode or empty-directory sealing.

Minimum free space was 89,983,307,776 bytes, above the continuous 10 GiB + 128 MiB
floor. No local native build, official checkout change, fork access, cache upload,
artifact publication or production runtime change was used. Only this report is
added after the successful frozen run; its trigger branch stays at the tested head.

The machine-readable result is [language-package-result-20261106.json](language-package-result-20261106.json).
The preparation READMEs and frozen input records retain their historical unrun
status; this later result closes only these exact eight cases.

## Still not proved

This is **not Game installation, actual pack/content overlay, Options language
selection, reload, live UI rendering, retained Guide/LAN refresh or runtime TOML
support**. The test-only byte view is not substituted for the Game pipeline.
Production package/asset coverage remains separate. In particular, the existing
whole-file locale overwrite problem is not fixed or validated by these cases.
See [the overlay compatibility design](language-overlay-compatibility.md).

