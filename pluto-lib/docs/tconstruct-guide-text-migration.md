# TConstruct guide text query migration WIP: do not activate

**Blocked by a concrete native side-effect difference.** Existing `wrap` measures
all resolved lines before drawing/hit traversal. Official `Font::advance` calls
`glyph_of`, which can allocate atlas entries and pending pixel uploads. The old
draw resolves/draws only 17 visible rows, while click measures only the chosen
row until its first hit. A later value fallback cannot undo those effects.
This source checkpoint is not safe consumer migration or ready for activation.
The ten prepared value fixtures do not establish cache/GPU equivalence.

This bounded consumer slice is based on central `8715b1232c0fca133fe012f35830e2108e653d59`
(remote equivalent `89744814b8d536a117e82f8ea3c8cdc9798faa95`). It uses the existing
Pluto Text conversion and Font layout APIs for the guide's main-page raw wrapping
and logical character hit queries. It adds one private TConstruct support module,
with no new shared API, hook, state, registry or native caller replacement.

## Preserved responsibilities

TConstruct retains root/title detection, pages and metadata, spread/cover layout,
coordinates, visible rows, navigation/history, click action execution and LAN
request/response behavior. Metadata is admitted as opaque content for this query;
it is not authorization. Material panels, tooltips and other embedded draw regions
remain on their existing paths. The full book is converted once, lazily, within
one handler so valid cross-page targets are not reinterpreted as single-page links.
No conversion happens on a cover or absent-page query. Nothing is cached across
invocations, resource reloads, world changes or screen changes.

The strict API is **not** a replacement for the legacy native Text/transport
contract. If document admission or wrapping fails, the exact old native splitter
is used. Unsupported actions/styles, invalid page numbers, large existing content
and work-budget rejection do not delete content. Query errors likewise use the
old resolved-logical-line traversal. Shared success returns only a page-link fact;
the existing action handler still decides what happens. Native fallback preserves
all previously represented ClickEvent variants without executing any in the adapter.

The intended partial delegation is blocked, not completed guide migration. An unrelated
unsupported page can cause the full document to use fallback. It is intentionally
not reencoded into Pluto's different wire schema. The retained fallback has the
same unbounded-input costs as the old path; this migration does not claim to fix
those. Strict conversion/validation adds bounded work per relevant handler, and
layout can additionally populate native metric/glyph caches while measuring lines.
No GPU/font-cache performance or exhaustion equivalence is established.

## Evidence and remaining limits

Five source regressions pass: inverse reconstruction of exact old click/draw
source bytes, manifest-only module addition, delegation/fallback scope, and native
fixture coverage. Reverse source hashes pin all unchanged geometry/action and
navigation code. These checks do not execute Font or Game.

Ten actual official Font/Text comparison cases are prepared in
`tconstruct/tests/guide-text-migration/`, against a tiny original `space` provider.
They compare shared and fallback results with the pre-migration native splitter
and logical traversal, including cross-page references, Unicode/legacy styles,
empty/zero-width characters, unsupported actions/styles, invalid links, document
and amplification limits, nonfinite coordinates/metrics and negative advances.
They are **not yet run** at this source checkpoint. Test preparation uses the
existing strict official source/lock/path verifier and records exact copied helper
bytes. It adds no mocked Game, renderer, window or network.

Only in the coordinator's bounded native slot:

```sh
python3 tconstruct/tests/guide-text-migration/prepare.py \
  --horizon-source APPROVED_CLEAN_OFFICIAL_SOURCE --output NEW_TMP_FIXTURE
cargo test --locked --offline --manifest-path NEW_TMP_FIXTURE/Cargo.toml \
  --lib consumer_tests:: -- --test-threads=1
python3 tconstruct/tests/guide-text-migration/prepare.py \
  --output NEW_TMP_FIXTURE --verify
```

Installed consumer plan/apply/typechecking, real rendering/input/audio, screen
lifetime and LAN response authority are **unproved**. Prior ordinary Pluto package
proofs do not become consumer proofs. This branch does not incorporate #945's
navigation changes: its existing turn-page and response hook composition blockers
remain visible and are not weakened here. #945 must be reconciled by its own
incremental navigation delta rather than by overwriting this consumer with an old
full tree. No existing PR state is changed and no feature is switched off.
