# Bounded native font layout facts

This native-host API supplies reusable font measurement, raw wrapping and
logical styled hit facts. It does not draw, execute a link, consume input, open
a book or identify a presentation. TConstruct retains its two-page spread,
positions, navigation regions, content and action policy. No consumer migration
or activation is part of this source group.

Native path: `horizon_client::pluto_guide_layout`. It is a public support module
in the shared client library, with no hooks, state extension or registry. The
input remains the existing bounded `pluto_inventory::guide::document::Document`.
No second style schema or font registry is added.

## Contract

`wrap(&Font, &Document, page, width)` measures one zero-based document page and
returns an owned `Wrapped` snapshot. Width must be finite and positive. The
exact native `Font::split` method runs on raw text; it is intentionally not
`split_formatted` or `wrap_components`. Changing that choice would change the
audited guide callers' legacy-code line boundaries.

- `native_lines()` borrows the exact raw native Text lines for a caller's draw
  operation. Empty/trailing lines, native space trimming and raw legacy codes
  retain the native splitter's behavior. Native splitting can discard empty
  source spans; this is not a lossless document serialization round trip.
- `width(line)` returns the captured native `Font::width_text` result. Native
  legacy resolution and bidi measurement are retained. Width is not a promise
  about the logical hit traversal's extent or visual glyph placement.
- `hit(x, y, line_height, first_line, visible_lines)` uses local text coordinates
  and caller-chosen visible row geometry. It resolves legacy codes separately
  within each raw line, then subtracts native advances in logical scalar order.
  It returns the first character leaving a negative remainder, exactly the
  audited native/consumer traversal. Right boundaries advance to the next
  character and zero advances are skipped. No wrap-width clipping is invented.
- A hit contains absolute wrapped-line index, resolved Unicode scalar index,
  character and the existing portable Style. A page link or opaque metadata is
  only a fact, not permission to execute it. This is **not visual bidi hit
  testing**, a byte/UTF-16 index, input admission or session ownership.
- Coordinates and line height reject nonfinite values; line height must be
  positive. Negative coordinates and positions outside visible/current lines
  produce no hit. A zero visible-row count is empty. Index overflow is an error.
- The result is a measured snapshot, not a live font/screen handle. It must be
  recomputed after relevant resources/content/layout change. It contains no
  Game, Font reference, actor, world, input or pending-response identity.

The one existing native Text converter is reused. Its internal native-style and
single-page conversion operations are factored out, keeping original admission
and error order, metadata policy and unsupported-style rejection. The adapter
does not accept arbitrary native Text and silently drop commands/URLs/fonts or
hover content. Callers wanting to import native Text must first use the existing
strict Document converter and handle its error.

## Resource bounds

Before native conversion, splitting or per-character style cloning, admission
counts at most 4,096 raw Unicode scalars and charges each potential native
Style/Span copy including its opaque metadata. The checked charge has a 4 MiB
ceiling and reserves four passes plus original-span overhead. This rejects
metadata amplification even if a document fits its 256 KiB wire limit.

There are at most 4,097 lines (all newlines plus the trailing empty line). Output
scalar/line limits are checked defensively, but they are not the first bound:
the input charge already constrains the native splitter's allocation work.
Nonfinite advances, aggregate metric magnitudes and measured widths are errors.
Finite native advances are not silently normalized or substituted.

The charge is a conservative logical work limit, **not an allocator memory
guarantee**. The caller's Font may populate its native metric/glyph cache; the
service does not cap that existing cache or validate a resource pack. A read-only
query here means no gameplay/presentation action, not no interior font caching.

## Actual official fixture and evidence limits

The fixture uses the complete official `horizon-client-render::font::Font`,
`Font::metrics`, `split`, `advance`, `width_text` and native Text from Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`. Its tiny `space` provider JSON is
original test data. It neither imports Mojang assets nor constructs a renderer.
Dependency/source identity checks reuse the existing locked guide fixture tool.

Eight tests passed for actual native provider parsing, exact split output,
Unicode/style/edge facts, raw-before-resolved legacy behavior, logical-vs-bidi
distinction, finite geometry, pre-split amplification limits and converter
equivalence against frozen production source from `38b07177`. The oracle is the
old converter's production functions without its test module, not a mock Text.

At exact source `0d67efc71cd65c0727f66ab2cc9b170c22e5c355`, preparation and
locked/offline source/path verification passed, followed by all eight layout
tests and all three existing converter tests in separate filters. No test was
ignored. Seven source/package contract tests also passed; their scope is
separate from native execution. Minimum free disk was 10,977,452,032 bytes,
above the 10 GiB +128 MiB floor, with no guard stop. Exact logs and the fixture
receipt are in [the validation record](guide-font-layout-validation.json).

Complete-package application/typechecking of this new module remains pending
central integration. The fixture proves only these font/codec queries on original
metrics data. It cannot establish rendering, physical input, current-book action
behavior, real guide screen lifetime or delayed-reply safety. Signed/large or
nonfinite provider inputs and rendered-font cache exhaustion were inspected at
source level but are not executed fixture cases.
The excluded lifetime experiment and ordinary verified v12 package are unchanged.

Reproduction commands, only in the coordinator's native slot:

```sh
python3 pluto-lib/tools/prepare_guide_text_fixture.py \
  --horizon-source APPROVED_CLEAN_OFFICIAL_SOURCE --output NEW_FIXTURE --include-layout
cargo test --locked --offline --manifest-path NEW_FIXTURE/Cargo.toml \
  --lib guide_layout_tests:: -- --test-threads=1
```

Preparation records the production module/converter/API, synthetic provider,
frozen baseline, native Font/bidi/Text sources and exact lock/path identities.
No original consumer hooks or Tinkers-specific layout are installed by this API.
