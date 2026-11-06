# Actual language-overlay baseline: passed

Exact `ce495a88afe32b6322703ad778263a90b2bb98a9`, [run 42195249104](https://github.com/sofia-gros/horizon-mods/actions/runs/42195249104), job 127816778163, official Horizon `624c4f62`.

**Eight native cases passed**, all exact names, with no ignored or filtered cases.
Actual cold compilation of client-core, mods, client-lib and the test fixture
was witnessed (about 295 seconds). Runner26 and source6 also passed. All 14
phases exited zero; the final receipt is `passed=true`, `verified=true`,
`error=null`. Minimum free space was 88,705,925,120 bytes.

## What this establishes

The real public `mod_content::build`, Package/Content and native PathBuf
ResourceSource/Language loaders were exercised on owned minimal roots. The
current Mod locale replaces the **whole** same-path lower locale, losing absent
lower keys. Cases also cover native namespace order, exact-locale English
fallback, malformed locale bytes, namespace conflict rejection, failures before
versus after output recreation, and rebuilding only owned output. The original
base and package are preserved in the tested cases.

This is a baseline result, **not a translation priority repair**. It does not
exercise selected/server pack preparation or filters, Game admission, Options,
startup/reload installation, live UI, retained Guide/LAN refresh or runtime TOML.
The minimal resource tables do not represent a complete playable asset bundle.

## First failure remains part of the evidence

[Attempt 1](https://github.com/sofia-gros/horizon-mods/actions/runs/42191906292)
at `14902fcc` failed before native compilation: the external fixture workspace
did not inherit the official root's local `naga`/ `wgpu-hal`/`libmimalloc-sys`
patches. Its false/verified receipt remains in
`experiments/language-overlay-ci/first-attempt.json`; verified there means source
preservation, not behavior success.

The reviewed fix at local `2c54a5a4` / remote `ce495a88` explicitly repeats only
those exact official paths, guarding declarations, manifest hashes/names and
metadata paths. The official lock identity contract remains intact. The Rust
fixture, template, runtime, controller and workflow were unchanged. No online
replacement dependency, automatic repeat or local native fallback was used.

## Evidence boundaries

The adjacent JSON preserves all required gates, exact names, compiler witness
hashes, lock/metadata identities, phase summaries and final seals. All checkout
bytes/file sets/non-followed symlink targets, fixture lock/metadata and generated
OUT_DIR were checked after the run. This does not claim modes or empty-directory
preservation. Full raw Cargo outputs were validated in CI before console
summarization; their hashes do not make ephemeral raw files retrievable later.

This closure changes documentation only. No runtime repair or consumer activation
is included, and the successful trigger remains fixed at `ce495a88`.
