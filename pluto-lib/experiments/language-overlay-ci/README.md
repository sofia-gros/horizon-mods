# WIP actual Mod language-overlay baseline CI

The first run failed before native compilation at offline dependency resolution.
Its eight Rust cases remain uncompiled and unrun. The corrected source below is
not yet retried. It does not modify the existing
Package8, i18n22 or installed-language host evidence or their trigger branches.

## What will be exercised

The frozen eight-case fixture from `01ccfa4f` (same fixture bytes on `783deeb1`)
uses the actual official `horizon_client_lib::mod_content::build`, Package,
Content and native `Language::load` with the native PathBuf ResourceSource.
The minimal facts/gameplay/items files are owned fixture assets, not substitute
Rust types or a playable resource bundle. All outputs are newly owned scratch
paths. Namespace-clash/Plan failures before output recreation and read failures
after recreation are deliberately distinct; no rollback promise is inferred.

The baseline records current whole-file Mod-over-lower-root replacement,
lower-only key loss, native namespace order, missing exact locale fallback,
namespace conflicts, invalid locale bytes and owned output rebuild. **It is not
a priority repair, selected/server pack merge/filter test, Game installation,
Options change, live reload, rendered UI, or retained Guide/LAN refresh.** Those
remain separate contracts. The public `mod_content::build` implementation is used
unchanged; no Mixins installation or native source overlay is involved.

## Exact identity and execution gates

Only the named public MOD trigger branch on standard ubuntu-24.04 is admitted.
Repository/branch/source-base, official URL/revision and official lock are fixed
before any fetch. The actual official Horizon `624c4f62` checkout must remain
clean including ignored/untracked files. The official lock is seeded, fetched
locked, then resolved offline permitting only official package identities and
one local fixture. The generated fixture explicitly inherits only the official
root's three exact local crates.io patches: `naga`, `wgpu-hal` and
`libmimalloc-sys`. Their root declarations and manifest bytes are source-pinned;
redirected paths, extra/version-based patches and substituted package names are
rejected. No registry replacement or additional online fetch is allowed. Metadata
requires the real local official manifest paths, including those three precise
`third_party` owners.
The actual client-lib dependency has default features disabled in the frozen
fixture manifest; it still brings the full native host library graph. This is
not the smaller core/mods-only Package8 compilation.

Required gates: 26 runner regressions, six exact-source guards, lock/metadata,
actual cold compiler witnesses for client-core/mods/client-lib and the fixture
`--test`, and all eight exact tests once with zero ignored/filtered cases. The
source suite's historical `run_now=false` describes its preparation checkpoint,
not a claim that this separately authorized controller has run already.

Raw logs are complete files consumed by strict validators. Console output is
bounded: per-phase byte count/SHA/exit, actual compile witness hashes, validated
test names, and the final receipt. The metadata receipt retains the complete
graph count/hash and exact four required local owners, not hundreds of repeated
registry paths; the full metadata file is still verified and sealed. A failed command prints only its bounded
16 KB tail. This avoids losing the final proof in a huge Cargo log; the summary
is not substituted for the raw test parser. Recompilation is detected after the
execution command completes, not intercepted before rustc could start.

Both source checkouts (all files, ignored files and non-followed symlink targets),
fixture/lock/metadata files and generated OUT_DIR files are checked after work.
These seals do not claim permission-mode or empty-directory preservation. Any
failure produces a partial receipt; success requires every gate and final
verification. One job, debug0/incremental0, fresh Cargo/target, continuous
10 GiB + 128 MiB floor and 2 GiB setup reserve apply. Timeout/disk failures stop
only the owned process group and verify it is gone.

No cache, artifacts, secrets, persisted credentials, private runner, deployment,
new permissions or fork source. Local native execution is not a fallback.
Source/Python checks and independent review must finish before trigger creation.

## First attempt and bounded correction

[Run 42191906292](https://github.com/sofia-gros/horizon-mods/actions/runs/42191906292)
at exact `14902fcc18a86d819fe48f0b9c4d687a3452d823` passed runner24 and source6,
then offline metadata failed with missing crates.io `naga`. The complete compact
receipt is preserved in `first-attempt.json`: `passed=false`, `verified=true`,
native compilation/tests not started. `verified` means final source preservation,
not a successful dependency or behavior gate.

Official `624c4f62` root `Cargo.toml:69-76` supplies three local patches that an
external fixture workspace does not inherit automatically. The fix changes only
fixture preparation and its exact dependency guards. The Rust fixture, original
manifest template, official lock, runtime source, workflow and controller remain
unchanged. The generated external workspace now repeats those exact official
patch paths rather than resolving replacement packages online. Two new Python
regressions cover accepted exact declarations and rejected substitutions. Existing
whole-graph lock identities, actual-path checks and final seals stay mandatory.

## Prepared source validation

The corrected 26 runner regressions PASS (0.031 s) and six source guards PASS
(0.111 s), separately from Rust, with the same strict Python-output parser. No local Rust, Cargo,
metadata, planner/apply, checkout copy or native host work is authorized here.
Independent review, source freeze and a new serial CI grant are required before
retry; the first failed trigger remains unchanged until that grant.
