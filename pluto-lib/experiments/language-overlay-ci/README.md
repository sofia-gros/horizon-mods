# WIP actual Mod language-overlay baseline CI

This is a separately queued, **not yet dispatched or natively executed** controller.
Its eight Rust cases remain uncompiled and unrun. It does not modify the existing
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
one local fixture. Metadata requires the real local official manifest paths.
The actual client-lib dependency has default features disabled in the frozen
fixture manifest; it still brings the full native host library graph. This is
not the smaller core/mods-only Package8 compilation.

Required gates: 24 runner regressions, six exact-source guards, lock/metadata,
actual cold compiler witnesses for client-core/mods/client-lib and the fixture
`--test`, and all eight exact tests once with zero ignored/filtered cases. The
source suite's historical `run_now=false` describes its preparation checkpoint,
not a claim that this separately authorized controller has run already.

Raw logs are complete files consumed by strict validators. Console output is
bounded: per-phase byte count/SHA/exit, actual compile witness hashes, validated
test names, and the final receipt. A failed command prints only its bounded
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

## Prepared source validation

Runner regressions: 24/24 PASS (0.020 s), including the CI's strict Python-output
parser. Fixture source guards: 6/6 PASS (0.131 s), against the pinned official
checkout. The first Python run passed all tests but two new witness prints
interleaved with unittest status lines; those regression calls now capture their
own stdout, and the entire suite plus its strict log parser were rerun. No Rust,
Cargo, metadata, planner/apply, checkout copy or native host work ran locally.
Independent review and the coordinator's serial CI release remain required.
