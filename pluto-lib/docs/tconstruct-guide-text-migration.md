# TConstruct guide text queries: raw/lazy migration WIP

This consumer slice is based on central `8715b1232c0fca133fe012f35830e2108e653d59`
(remote equivalent `89744814b8d536a117e82f8ea3c8cdc9798faa95`). It reuses Pluto's
strict Document/Text converter and adds raw/lazy native Font operations to the
existing layout module. The consumer delegates main-page raw wrapping and logical
character hit facts. No hook, state or registry is added; one private consumer
support module is installed by the candidate manifest.

**This is source-only WIP, not validated consumer activation.** The earlier eager
candidate is preserved at remote `46673babc6c312bf8350f94455cecf5116c6f515` / local
`233774f4704c9b3b2556cf9bf47eab1944e9b303`. It must not be used: eager `wrap`
premeasures all raw characters and every resolved line. Native `Font::advance`
can allocate glyph/atlas entries and pending pixels. A value fallback cannot undo
those effects or restore original allocation order.

## Exact operation boundary

| Path | Native operation sequence |
|---|---|
| Original draw | `Font::split` on the raw page, then existing draw calls for at most 17 visible lines |
| Original click | `Font::split`, resolve one selected raw line, `advance` in logical order until the first negative remainder |
| Rejected eager `wrap` | All-raw `advance` preflight, `split`, every-line `width_text` (legacy/bidi), every-line resolution and advances |
| New `split_raw` + draw | Pure bounded admission/conversion, one `Font::split`, unchanged visible draw calls |
| New `split_raw` + `hit_native` | Pure admission/conversion, one `Font::split`, resolve selected line, native advances until the first negative remainder |
| Strict API failure | Failure occurs before any Font access, then the original splitter/traversal; no post-query error replay |

`RawLines::hit_native` returns an owned native `Style` fact, like the existing
native Text line view. It is not another portable schema or permission to execute
its ClickEvent. It intentionally preserves native signed/nonfinite arithmetic,
rather than silently introducing a normalization or a fallible post-traversal
phase. Width/page/work admission is complete before `split`. No `width_text`,
metric preflight or off-row resolution occurs in these new operations. Existing
eager `wrap` behavior/API stays unchanged for callers explicitly requesting it.

Source evidence at official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`:
- `crates/horizon-client-render/src/font.rs`: `split` at 1231, `advance` at 1048,
  `width_text` at 1068, `glyph_of` at 770 and `Region::place` at 556. The latter
  records pending pixel patches and can report a full region.
- `crates/horizon-client-core/src/text.rs`: `slice` at 533 reconstructs only
  spans. Thus loss of Text's private source-JSON cache during strict conversion
  does not alter raw split lines. `resolve_legacy` at 454 preserves the supported
  style subset and does not access Font.
- `source-materials-book-click.rs` / `source-materials-book-draw.rs` at the base
  above. Inverse source guards reconstruct their exact original hashes, so the
  caller's unrelated actions and geometry are not inferred from a modeled screen.

This source comparison supports matching native call sequence, not measured live
GPU/cache equivalence. Document construction adds bounded serialization/cloning
work before queries. The native Font's own cache effects remain native, not pure.

## Consumer compatibility

TConstruct retains root/title detection, pages and metadata, spread/cover layout,
coordinates, visible rows, navigation/history, click action execution and LAN
request/response behavior. Metadata is admitted opaquely, not interpreted as
permission. Material panels/tooltips and other embedded draw regions remain on
existing paths. Full-book conversion is lazy and happens at most once within a
handler, retaining cross-page target numbers. Covers and absent-page queries do
not convert anything. There is no cross-invocation cache or presentation handle.

The strict API is not the legacy native Text/transport contract. Rejected styles,
actions, page numbers, existing large content or budget failures retain original
native fallback, including action facts. Nothing is silently removed/reencoded.
One unsupported page can cause the whole document to use fallback; this is
partial delegation. The fallback's original unbounded-input costs remain.

## Validation status

Eight source regressions passed: exact inverse click/draw reconstruction, module-only
manifest addition, delegation/fallback scope, no extra/replayed native query calls,
prepared fixture coverage, original PNG bytes/intended full-region assertions,
and atomic fresh asset-root creation before any native loading. A peer read-only review found no serious issue in
the corrected sequence; it ran no build or test.

All twelve actual official Font/Text cases **passed** in the dedicated CI run
`42171288439` at exact head `49a2401e` on 2026-11-06. Ten use
original `space` provider data and compare shared/fallback results with the frozen
native traversal: cross-page links, Unicode/legacy styles, empty/zero advances,
unsupported actions/styles, invalid links, document/amplification limits, invalid
widths, nonfinite coordinates/metrics and negative advances. Two use public native
`Font::load` and an original 16x8 RGBA bitmap to compare pending CPU pixel patches,
selected-row queries and subsequent missing-glyph behavior with normal versus
one-glyph atlas capacity. Expected patch counts, retained first glyph and missing
second glyph/advance are asserted independently, so equal results alone cannot
accidentally pass without reaching the full-region branch. They do not construct a window or GPU. Successful CPU
patch comparisons do not prove real HUD drawing/upload/lifetime/LAN.

Only under the coordinator's bounded native grant:

```sh
python3 tconstruct/tests/guide-text-migration/prepare.py \
  --horizon-source APPROVED_CLEAN_OFFICIAL_SOURCE --output NEW_TMP_FIXTURE
cargo test --locked --offline --manifest-path NEW_TMP_FIXTURE/Cargo.toml \
  --lib consumer_tests:: -- --test-threads=1
python3 tconstruct/tests/guide-text-migration/prepare.py \
  --output NEW_TMP_FIXTURE --verify
```

Preparation reuses exact official source/lock/path guards and records helper/API
bytes. It does not patch the host. The dedicated CI ran real official dependency
metadata, compilation and the twelve-case fixture; it did not run the full native
consumer planner/apply/type gate. Prior Pluto package proofs do not become consumer
proofs. Screen lifetime and LAN response authority remain unproved.

This branch does not incorporate #945 navigation. Its old turn-page and response
composition blockers remain visible and no guard is weakened. Reconcile #945 by
its own navigation delta, not an older full tree. No feature or existing PR state
is changed by publication of this WIP checkpoint; it is not a main-ready branch.

## Historical local execution preparation review

The source review tightened the CPU fixture's intended-branch assertions and
validated the original PNG signature, chunk CRCs, dimensions and every decompressed
pixel with a pure regression. Fixture verification now compares the copied API's
complete file set and bytes with the frozen production API, in addition to the
existing official source/lock/local-dependency identity checks. It also seals the
preparer and consumer draw/click/manifest sources; these are not typechecked by the
Font-only fixture and remain explicit wider-integration limits.

Do not run the documented commands as an unguarded batch. A granted run must record
an exact clean local commit, use a new fixture/log directory under `/tmp`, the
coordinator-selected existing shared target, `--locked --offline`, debug info 0,
incremental off and at most two jobs. Monitor workspace free space continuously
against 10 GiB +128 MiB, terminate the process group on crossing the floor, and
verify source/lock/fixture identities after terminal status. Record all twelve
named results, ignored/filtered counts and logs separately from the eight pure
checks. `/tmp` capacity and workspace capacity are separate budgets. Do not delete
or change a previous sealed fixture, native stage or shared artifact to force a
pass. At that local review, approximately 5 MiB headroom did not show that
metadata/linking could safely fit; local native execution was not authorized. The
subsequent isolated CI proof below does not grant local execution.

The `RawLines` value is a snapshot of split native lines, not a font/screen handle.
The caller must choose when to re-split after content, font or layout changes.
`hit_native` uses the explicitly supplied current Font; it neither proves that it
matches the splitting Font nor owns a world/window/input lifetime. Existing
TConstruct use keeps split and hit within one synchronous handler.

An independent read-only review of the corrected fixture, identity verifier and
raw/lazy operations found no serious additional issue. It confirmed separate
old/new CPU Font instances, explicit normal/full region assertions and an oracle
that invokes native traversal without Pluto. The reviewer ran no execution,
metadata or build and did not claim visible-row draw/UI/GPU/package validation.


The [read-only preflight review](guide-text-migration-preflight-review.json)
records unchanged LAN/content/navigation source hashes and the exact old fixture
dependency comparison. Its prior executed Font test binary was 12,493,320 bytes,
already larger than the approximately 7 MiB available headroom when reviewed.
That is evidence against an assumed tiny run, not a fresh output upper bound.
The old frozen fixture inputs still match their receipt; no native metadata was
started and no previous artifact was deleted by this worker. Original source-only
and new twelve-case execution claims remain separate.


## Fresh fixture asset roots

Before native execution, a further read-only review found that the PID/counter
asset directory used `create_dir_all` without rejecting leftovers. The fixture now
atomically creates that root with `create_dir` before writing or loading anything.
An existing directory or symlink fails closed and is not overwritten or removed.
The existing cover/missing-page test also checks `AlreadyExists` and unchanged
asset bytes on a second creation attempt. There are still twelve native cases;
all twelve subsequently passed in the isolated CI proof below. The added pure
source guard also passed, bringing that separate count to eight. This is a test-only correction, not changed production behavior.

The prior terminal Font executable was subsequently archived and verified by the
separate storage recovery procedure; its source, hashes and logs were preserved.
After the next prioritized API test, available workspace headroom was still only
about 13 MiB, close to that old executable's size before allowing for new library
or temporary object output. Local native/Cargo/metadata execution remained ungranted.
The earlier c4ba fixture source must not be substituted for this corrected cutoff.


## Dedicated CI result, exact frozen inputs

The [accepted CI run](https://github.com/sofia-gros/horizon-mods/actions/runs/42171288439)
completed successfully at `49a2401eb3957e0766c8e68adbc45d69882aa793`, tree
`01cfb56403cdb3bdee96ad5ea47f8ae0b3a03437`. It used the unchanged corrected
fixture source `59c0651b` and official Horizon `624c4f62`, with the exact official
Cargo.lock and Rust 1.95.0. Earlier `3a0186ee` and `06aee105` runs were cancelled
and are not successful evidence.

- Eight source regressions passed.
- Twelve named native cases passed, with zero failed or ignored and eleven other
  compiled tests filtered out. Those eleven were not executed by this run.
- The cold, locked dependency fetch preceded offline preparation. The log records
  actual `rustc` invocations for `horizon_client_core`, `horizon_client_render` and
  `pluto_guide_text_tests`; native compilation took 261.522 seconds.
- Exact source/API/lock checks passed after compilation and after tests. Generated
  native OUT_DIR inputs were also unchanged across execution.
- The final receipt recorded 88,776,589,312 minimum free bytes against the
  10 GiB +128 MiB floor. The guarded hosted runner did not consume the constrained
  local native build budget.

The [machine-readable result](guide-font12-ci-result-20261106.json) preserves the
exact head, all twelve names, phase exits, final receipt, source-seal hashes and
superseded-run distinction. This documentation update changes no tested runtime
or fixture input.

This is positive proof of the split/selected-logical-hit delegation and fallback
against actual native Font/Text, including original synthetic CPU glyph patches
and full-region missing-glyph fallback. It is not full consumer installation,
visible 17-line drawing, GPU upload, performance, screen lifetime or LAN response
proof. Existing #945 and LAN composition work remains separate. The consumer WIP
must not be called main-ready solely on this fixture result.
