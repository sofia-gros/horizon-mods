# Shared tinted drawing mechanics

PlutoLib's public client-library support module `horizon_client::pluto_render`
is a bounded rendering compatibility adapter. It owns UV conversion, repeated
GUI tiles, unshaded entity vertices and packed-light adaptation. It does not
register render hooks, discover machines, select fluid content, or load assets.

Consumers keep their textures and fallback keys, tint/luminosity tables, potion
metadata policy, UI layout, capacity-to-height calculation, cuboid bounds, mask
shapes and visible faces. In particular, this adapter does not turn ordinary
custom fluid blocks into native liquid meshes or change world-fluid physics.

## API and ownership

- `sprite(&HudAtlas, key)` returns an opaque borrowed `Sprite`. Its
  `logical_size()` returns two pixel dimensions. A view must not outlive its
  atlas borrow or be retained across resource reloads. Missing sprites return
  `None`; fallback selection belongs to the consumer.
- `crop(batch, sprite, source, source_extent, destination, color)` appends a
  tinted GUI crop. Source and destination rectangles are `[x,y,width,height]`.
  Source extent is explicit, not implicitly the sprite's logical size. This
  preserves masks whose coordinate grid is 16×16 even for other logical sizes.
- `tiled_bottom_up(batch, sprite, [left,bottom,width,height], tile, color,
  quad_budget)` emits rows bottom-up, columns left-to-right. Each partial tile
  takes the top-left portion of the sprite. UV scale uses its logical size.
  Preflight uses the same float32 stepping as emission; no change to valid
  source rounding or vertex order is intended. Budget is capped at
  `MAX_TILE_QUADS` (4096). Current consumer calls use a budget of 64 and at most
  49 quads per fill.
- `cuboid(out, sprite, origin, bounds, faces, uv_span, color, light)` appends
  four entity vertices per selected face. Origin is already camera-relative;
  bounds `[x0,y0,z0,x1,y1,z1]` are final block units. Faces retain caller order,
  including repeated selections, with a six-quad limit. `TOP`, `TOP_AND_SIDES`
  and `ALL_FACES` are convenience selections. Zero-height bounds are valid.
  UV span uses logical pixels; its arithmetic deliberately preserves the mesh
  path's multiply-after-division ordering, distinct from GUI crop arithmetic.
- `minimum_block_light(packed_world_light, minimum)` preserves the sky nibble,
  raises the block-light nibble to the supplied minimum (0..15), then uses the
  official host `vertex_light` conversion. Pluto does not choose luminosity.

Color is already packed `0xAABBGGRR`, including alpha. It is not repacked or
shaded. Entity layer, normal and glint UV are zero. The adapter does not change
blend runs, depth state or preexisting output. Invalid/nonfinite inputs,
inverted bounds, invalid extents and excess budgets return `DrawError` before
appending anything. Zero-area GUI destinations append nothing. Requests are
bounded even for subnormal tile sizes. Source crops are not clamped to the
atlas region: consumers own selection, including intentional UV spans.

The five migrated consumer helpers deliberately omit a draw on `DrawError`.
Their valid source-owned dimensions and budgets satisfy this contract. Source
luminosity is asserted to remain within 0..15. No existing consumer hook ID,
target, cancellation behavior, machine discovery or culling was changed.

## Why not the host's high-level native draw helpers?

On official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, `native_sprite`
uses white tint, while `BlockEntityDraw` applies directional shading and RGB
color conversion. Its callback path also has different discovery, culling and
caching rules. They are not equivalent replacements for these existing tinted,
unshaded output paths. This extraction keeps their original hook bindings.
The public boundary still accepts host batch/output types, and those bindings
remain consumer-owned; it does not guarantee that every future host update
will require changes exclusively within Pluto.

## Validation and reproduction

The candidate targets official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`
and Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`.

Passed for this source group: four source-boundary checks and eleven selected
existing consumer checks (Copper Can display, complete potion metadata, Melter
remote tooltip, Scorched Basin machine paths). These are source checks, not
native vertex equivalence or rendering proof.

`tests/native/rendering/` contains eight native tests using the **real official**
`HudBatch`, `Rect`, `HudVertex` and `mesher::Vertex`. `oracle-sources.json` pins
five original consumer files to commit `1e870214` with SHA-256 digests and
records the small parameter/import adaptations to the frozen oracle. Tests
compare float32 bits and every vertex field, output prefixes and blend state:

- widths 12/16/52/106, integer/fractional heights, nonzero origins, layer offsets;
- the four mask rows with an explicit 16-unit source extent;
- zero/partial/full cuboids, top-only/top+sides and all faucet orientations;
- face selection/order and all 4096 world-light/minimum combinations;
- invalid, nonfinite, zero-area, subnormal and over-budget inputs without
  partial output.

Prepare a disposable fixture from a clean approved official git checkout,
without modifying that source tree:

```sh
python3 tools/prepare_render_fixture.py \
  --horizon-source /path/to/approved/official/checkout \
  --output /path/to/new/fixture
```

The fixture imports the actual adapter source and depends on official
`horizon-client-render` under that checkout, not mock structs or the full game
harness. Preparation requires the configured exact revision, official origin and
a clean checkout, copies the official lockfile and local patch overrides, and
resolves metadata offline. It rejects every new dependency identity except the
exact local fixture package, comparing name, version, source and checksum.
The official lock and all fixture inputs are hashed. `--verify-lock --output
/path/to/fixture` rechecks those guards before/after execution. After receiving a coordinated build slot and sufficient disk headroom,
run `cargo test --locked --offline --manifest-path /path/to/new/fixture/Cargo.toml --lib` with the
approved environment/cache. The preparation command resolves metadata, but does not compile or run tests.
Two additional Python guard tests cover dependency drift and dirty/wrong official
source identity.

The native equivalence fixture **passed all eight tests** after a guarded
resumption, using the exact adapter source at `f651889e` and the official locked
dependency identities. The later complete shared
Pluto package at `0dd927ef` passed official plan, preflight, apply and host
library+binary checking. Its rendering source hash exactly matches this module. Full TConstruct
bundle compilation and in-game visual checks, resource reload and local/remote
rendering remain separate later gates.


### First bounded native attempt

The guarded, locked/offline fixture attempt at source `f651889e` stopped during
native dependency compilation after 43.55 seconds. The free-space guard was
10 GiB + 128 MiB; final free space remained about 10.12 GiB. The dedicated Cargo
process group was interrupted, with no Rust error and no tests executed.
Official source/lock/input identities passed again after the attempt. This is
an environment interruption, **not** a passed or failed vertex-equivalence run.
See [machine-readable evidence](rendering-proof.json). All build artifacts were kept. The next paragraph records the later successful
resumption, without discarding this first attempt.


### Successful locked native resumption

After verified cleanup restored headroom, the same fixture and shared cache
were resumed under the same continuous guard, debug=0, incremental=0 and two
jobs. All eight tests passed in the 38.52-second build/test attempt. The 4096
light cases, complete valid HUD/mesh vertex fields, crop coordinate distinction,
output prefixes and invalid-input atomicity passed against real official types.
Final free space was 11,106,922,496 bytes. Source/lock/input identities passed
again afterward. The fixture emitted one harmless unused-function warning for
`sprite` (lookup is type-checked but tests construct their synthetic regions
inside the module). This is a native mechanism equivalence proof, not a GPU,
resource-reload, full TConstruct bundle or gameplay proof.

## Heat, fire and gauge consumer follow-up

A separate follow-up migrates five existing crop blocks to the same unchanged
API: heat bars, fuel fire, the Melter heater/fuel panel, the Melter gauge and the
Smeltery/Foundry gauge. Texture keys, source pixel selections, panel modes,
fire-height arithmetic, empty-height returns, missing-sprite omission and
screen positions remain consumer-owned. No shared runtime/API, manifest or hook
is added or changed by this follow-up.

`crop-oracle-sources.json` freezes the prior three source files at `a10de5e0`.
Two additional real-type native cases compare all affected crop bodies against
that reference, including four heat states, empty/fractional/full heat heights,
all fire heights 0..14, both panel modes and both gauges, with normal and
non-square sprite dimensions. Two source-boundary tests pin caller coordinates
and empty behavior. The earlier eight-test proof remains valid for its exact
source. The expanded fixture at consumer commit `032045e9` passed all ten native
tests in 1.50 seconds using the same official dependency identities, and all
19 selected source checks passed. See [follow-up evidence](render-crop-proof.json).

The consumer-only follow-up is outside the `0dd927ef` complete-package cutoff;
that check covers its unchanged shared runtime, not the full new consumer
bundle. In-game visual and complete consumer bundle checks remain separate.
No new shared API, manifest or hook was introduced.

## Remaining machine panel crop follow-up

The next consumer-only group moves the remaining raw Rect/UV arithmetic from
`source-smeltery-draw.rs` to the same crop API: empty input icon, hover highlight,
three mode icons, scalable panel border/scroll regions and active/padding slot
backgrounds. No new API, shared runtime source or hook is needed. The consumer
retains all texture keys, cursor/empty checks, column/row/page calculations,
scroll thumb progress and pixel selections. For nonempty panels, the unchanged
layout guarantees positive region dimensions; zero-count panels still skip
those regions. Missing sprites are still omitted individually.

The panel oracle freezes exact old/new draw prefixes at reference `8443c8c8`,
substituting only Game inputs and sprite lookup with explicit test inputs and
actual official Rect values. Both paths use the unchanged source layout. Two
added native cases cover all mode/hover/empty combinations, absent sprite
combinations, column transitions, overflow paging, last-row padding, clamped
large offsets, fractional origins and non-square sprite sizes. No host renderer
types are mocked. The expanded fixture at consumer commit `703302c9` passed all 12 native tests,
including 272 added panel combinations, in 1.50 seconds. Source and official
lock identities passed before and after execution. Twenty-two source checks
passed; one separate standalone layout Rust test was skipped before toolchain
environment loading and is not counted as passed. See
[panel evidence](render-panel-proof.json). This is separate from the prior
ten-test proof and does not verify the complete consumer bundle or gameplay.

## Station, builder and worktable crop follow-up

Eight remaining crop blocks in `part-draw.rs` now reuse the unchanged shared
API: worktable choice backgrounds, generic station/builder parts, translucent
item cover, beam/decoration regions, rename field, selected/hovered layout
buttons, empty-input icons and builder scrollbar. No recipe, guide, preview,
item, naming, input or station-layout behavior moves to Pluto. A normalized
source-boundary check confirms the entire file outside these eight crop blocks
is byte-preserved; choices inside the blocks have explicit source guards.

The frozen old/new block fixture uses actual official Rect/HudBatch types and
real Pluto drawing code. Two additional native cases cover missing sprite keys,
selection/hover priorities, both empty inputs, scrollability, clamped/fractional
scroll progress, both beam themes, decorations, scaled widths and translucent
alpha. Source coordinate choices and texture keys remain consumer-owned.
At consumer commit `c17d168b`, all 14 native tests and 24 selected source checks
passed. Source and official dependency identities were verified before and after
the 1.50-second cached fixture run. See [station evidence](render-station-proof.json).
The shared renderer is unchanged and covered by the later `999468ef` package
proof; this new consumer group is outside that cutoff. Earlier proofs retain
their own exact scope. Full consumer bundle and in-game checks remain separate.

## Side inventory and information-panel crop follow-up

Two final crop closures in `source-side-draw.rs` and `builder-info-ui.rs` reuse
the existing API. All text, guide, recipe, viewport/column/row/padding/scroll
policy and every line outside those closures and sprite lookup remain
byte-preserved. No new hook, state or rendering API is introduced.

The actual information-panel callers use heights 83 and 184 with width clamped
to 80..126. The side inventory keeps at least three columns, including count
zero, so its non-slot frame regions retain positive area. Two native cases
compare the exact old/new geometry bodies using real renderer types, including
missing sprites, row transitions and overflow paging, fractional origins and
widths. The unchanged side title font draw is excluded from the geometry oracle
and protected by the full non-crop source comparison. At consumer commit
`08854c38`, all 16 native tests and 26 selected source checks passed. The cached
locked/offline run took 2.00 seconds; exact official source/dependency and fixture
input identities passed before and after execution. See [side-panel evidence](render-side-proof.json).
The shared runtime remains unchanged. This does not verify the complete consumer
bundle, font rendering, GPU output or gameplay.
