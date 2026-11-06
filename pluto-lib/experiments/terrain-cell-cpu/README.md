# WIP: actual native CPU mesh fixture

**Source prepared only. No native compile, test execution or timing result yet.**
This test-only overlay is not in ordinary PlutoLib packaging. Production terrain
adapter bytes remain the immutable `d63a7c8e` cutoff. Full installed apply/type
validation is coordinated separately.

The fixture uses real official `Assets::load`, `StateModels::from_facts` baking,
`Sprites`, `FaceVisibility`, `MeshTables::new`, `Biomes::load`, resident `Chunk`,
`MeshScratch` and installed `Mesher::mesh_reusing`. It never models those types.
Inputs are small original synthetic JSON and generated PNGs, not downloaded or
fork assets. BlockFacts are explicit synthetic native structs. RenderAssets are
assembled from the real native model/sprite outputs, not the complete gallery
loader. Neither production vanilla resource loading nor GPU output is proved.

`prepare.py` takes a complete production `.hnmod`, verifies the exact terrain
module hashes, and adds one test-only module. It regenerates an unmodified native
baseline body from the pinned official source, changing only the receiver/name.
It records the bundle, native source, overlay and resource hashes. Generated
resources total about 7 KiB for ordinary and deliberately ambiguous models; the
bundle extraction is about 1 MiB, not an official source workspace copy.

Run the fixture only after the coordinator grants the serial native slot and
sufficient free space. The ongoing floor is 10 GiB + 128 MiB, not just a start
check. Preparation itself runs no Cargo metadata, planner, apply or native build.
Do not apply into a frozen/shared proof stage. Use the centrally approved test
stage or its approved bounded overlay mechanism.

Preparation:

    python prepare.py --official PRISTINE_624C --bundle COMPLETE.hnmod --output NEW_OUTPUT
    python prepare.py --output OUTPUT --verify

The complete overlay plan must pass all production guards, including both terrain
guards, before apply. Also run `python check_plan.py OUTPUT COMPLETE_PLAN.json`.
Verify it contains exactly one extra module named
`pluto_terrain_cell_cpu` from this generator, no extra injection and unchanged
production payloads. Actual application/type/test remains a separate gate.

Run the single non-skipping native test twice, in **separate processes** because
the provider registry intentionally freezes once:

    PLUTO_TERRAIN_FIXTURE_RESOURCES=OUTPUT/resources
    PLUTO_TERRAIN_FIXTURE_MODE=no-provider  # then registered
    PLUTO_TERRAIN_PROFILE_LABEL='test; opt-level=0; debug=0; incremental=0'
    PLUTO_TERRAIN_TIMING_ITERS=64
    cargo test --offline --locked -p horizon-client-render --lib \
      mesher::pluto_terrain_cell_cpu::tests::real_native_mesh_differential_and_timing \
      -- --exact --nocapture --test-threads=1

Those environment assignments must be exported or attached to the command. Use
the approved shared target, no RUSTFLAGS, at most two build jobs and disabled
debug/incremental. Record the actual Cargo profile and optimization overrides;
the label must not imply optimized production performance if the run is
unoptimized. An optimized performance run needs its own disk/build approval.

## Intended assertions (not yet executed)

- No provider and provider returning None: bitwise native vertex streams and
  exact lights, block entities, sprite animation notes, visibility, bounds,
  packed data and tint arrays, including after native solid sorting.
- Claimed half-height geometry: changed surface coordinates with retained native
  pass/culling counts, light/entity collection, real animation notes and four
  smooth/shader-pack lighting combinations.
- Invalid coordinates, exhausted input budget, ambiguous baked faces,
  unprepared identical resource owners and a controlled monotonic revision
  change: entire output returns to native semantics.
- Fallback replay: compare own-cell/padded block and light data, solid rows and
  every observable leaf-mask interior decision. Reuse the same mesher, scratch
  and output for changed padding/neighbors, an empty early return and new data.
  Active leaf-cache modes are included. After the dense input-budget fallback, a later
  valid single cell must resume its half-height claim rather than remain native. Native private cache layout and buffer
  capacity are not semantic equivalence requirements; capacities are reported.
- Original native code's missing-neighbor padding is exercised, not assumed to
  be air. Assets missing or models not baked fail the test rather than skip it.

Timing uses warm reusable buffers, alternating native/adapter order over seven
samples, with per-section nanoseconds and median ratios printed as
`PLUTO_TERRAIN_CPU` JSON. Cases include no provider, registered ordinary blocks,
provider-no-claim and mixed claimed geometry. This is a diagnostic measurement,
not an assertion against a noisy machine-dependent threshold. It does not prove
production-frame performance, GPU upload/display, flow policy, fog or actual
TConstruct activation. Stale/invalid fallback intentionally performs extra work;
semantic equivalence does not promise zero replay overhead.

The scheduled `run.py` wrapper enforces the continuous disk floor, serial
no-provider/registered processes, an explicit unoptimized test profile, and an
exact one-test completion plus all expected timing records. A zero-test or skipped
run cannot pass its evidence check. It only consumes an already approved/applied
stage and shared target; it never applies or downloads sources. Run it only with
an explicit native-slot grant. Logs are exclusive-create to preserve failures.

The synthetic case does not exercise CTM/Fusion overlays, actual vanilla resource
packs, native water/lava shading or live renderer upload. The active leaf-cache
fixture uses explicitly synthetic native block facts, not a claim about vanilla
bricks. Allocation capacities/cache occupancy may differ after fallback; semantic
outputs and subsequent reuse must remain identical.
