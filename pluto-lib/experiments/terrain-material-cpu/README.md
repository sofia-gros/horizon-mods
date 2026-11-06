# WIP: material-aware actual native CPU fixture

Status: source-only preparation; **native unrun**. No local formatter, planner,
apply, Cargo metadata, compilation, test or timing measurement was run. This
excluded experiment changes no production module or ordinary package.

Fixed production head: `43f174273cd5f1b9cf3acd1d26abab8838025510`.
Its ordinary archive is `cd6de18cacbe7c40218ec817671bf3842aef3be0d586c613bb788babafbd3a76`.
The previous successful material/facade CI proves only its own type/test scope.
It does not prove this updated fixture. The earlier d63/v14 CPU fixture was also
unexecuted; no old CPU result is carried forward.

## Intended proof, not current results

One test is run in two fresh processes, `no-provider` then `registered`, because
first meshing freezes the one provider registry. Missing inputs fail instead of
skipping. The actual official `Assets`, `StateModels`, `Sprites`, `FaceVisibility`,
`MeshTables`, `Biomes`, `Chunk`, `Mesher` and `MeshScratch` types are used. Model,
PNG, collision and biome inputs are tiny original synthetic data. `BlockFacts`
are explicit actual native structs. `RenderAssets` uses actual baker/sprite
outputs, not the complete production resource/gallery loader.

The full official `mesh_reusing` body is the fallback oracle, mechanically
renamed with its receiver explicit. Its complete source and body SHA are checked.
No formatter, body hunk or substituted mesher implementation is used. The old
no-provider/no-claim, half geometry, ambiguous/unprepared inputs, dense budget,
stale revision and invalid-coordinate cases remain. Native fallback compares
bitwise vertex streams and every output field, before/after sorting. The same
mesher, scratch and output are reused after fallback across changed neighbors,
ordinary, empty and valid-claim sections. Padded blocks/light, solid rows and all
observable leaf-interior decisions are compared. Allocation capacities are
reported separately and need not match.

New intended material assertions:
- Actual `ParsedModels::load` plus `bake_standalone` prepares a second texture.
  Primary, secondary and per-face mixed choices preserve corresponding native
  geometry/light/color facts while selecting independent atlas layers and real
  animation notes.
- Tint removes only the green channel before native shading; other packed
  channels are compared with an otherwise identical untinted case.
- Top-oriented geometry occupies the upper half; this is render-only, not
  upward fluid physics or Mantle contact-surface parity.
- Actual opaque partial-UV models accept within-envelope UVs and reject a wider
  range. Incompatible shade, absent model/appearance, geometry-only provider with
  explicit appearance, stale revision and invalid coordinates fall back to the
  complete native section and replay observations.
- Actual Fusion loader assigns rules to a synthetic full-layout standalone
  model. A smoke case requires real tile layers rather than the primary/full
  sheet. It is not exhaustive Fusion predicates, overlays, animation or layouts.

Timing retains seven alternating warm-buffer samples, actual profile labels and
raw per-section times. There is no fabricated result or production performance
threshold. An unoptimized result is not optimized-game performance.

## Preparation and later gates

`prepare.py` accepts only the exact archive above, verifies production sources,
and adds one outer-`cfg(test)` module. It generates original resource variants,
records preparer/template/native/production/resource hashes, and rejects changed
bytes, symlinks and additional files when verifying. `test_source.py` is pure
Python and reads exact official source; it does not perform native preparation.

A later separately approved CI must build the fixed official CLI, plan the
complete overlay, pass all twelve ordinary source/composition guards plus
`check_plan.py`, then apply/type-check and compile the actual renderer test crate.
Compile and two executions must be separate with source/lock/generated-input
seals, exact named-test counts, no silent compile during execution, disk/PGID
limits and partial failure receipts. No CI workflow or trigger is added here.
The official source/dependency ancestry is a CI responsibility, not established
solely by a `.horizon-revision` marker or selected-file source hashes.

Exact test:
`mesher::pluto_terrain_material_cpu::tests::real_native_mesh_differential_and_timing`.
Environment: `PLUTO_TERRAIN_FIXTURE_RESOURCES=OUTPUT/resources`,
`PLUTO_TERRAIN_FIXTURE_MODE=no-provider` then `registered`, actual
`PLUTO_TERRAIN_PROFILE_LABEL`, bounded `PLUTO_TERRAIN_TIMING_ITERS`.

## Not established

There is no GPU execution, real-resource screenshot, production-asset proof,
installed TConstruct consumer, seventy registered fluid families or fluid
simulation proof. This is **not TConstruct activation**: an actual consumer still
needs startup content plus standalone-model registration before resource assembly,
one material provider before the first mesh, and the current registry-bound
preparation adapter. Those missing integration steps are not supplied by a test
module or a successful library type check.
