# Real native CPU terrain fixture: bounded proof

The corrected [run 42192378071](https://github.com/sofia-gros/horizon-mods/actions/runs/42192378071)
passed at exact source `1ac143d840eaf3dde9e108ada3f5f2ec9f4c451f`.
This closure adds documentation only and does not trigger a new workflow.

## Verified inputs and execution

- Official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and
  Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`.
- Unchanged production terrain/facade head `43f174273cd5f1b9cf3acd1d26abab8838025510`,
  archive SHA256 `cd6de18cacbe7c40218ec817671bf3842aef3be0d586c613bb788babafbd3a76`.
- All 16 required gates passed: 70 Python checks, exact package and generated
  fixture, official plan, all 12 ordinary guards and the extra fixture guard,
  official apply, actual host library/binary type checks, and actual renderer
  unit-test compilation followed by sealed-binary execution.
- Both native material contract tests passed:
  `prepared_texture_metadata_can_differ_without_changing_native_attributes` and
  `every_nontexture_attribute_mismatch_rejects_the_template`.
- `mesher::pluto_terrain_material_cpu::tests::real_native_mesh_differential_and_timing`
  passed once in each of two independent processes: no-provider and registered.
  Four and 51 replay receipts respectively were validated.
- Final source, lock, generated output, resource and binary checks passed.
  Minimum free space was 87,837,351,936 bytes, above the 10GiB+128MiB floor.

This uses genuine native loader, baker, owner and mesher types with original tiny
synthetic resources. RenderAssets is assembled from actual StateModels/Sprites
outputs; this is not a full RenderAssets::load/gallery or production asset test.
The unchanged full native mesh_reusing body is the fallback oracle. Comparisons
include bitwise vertices, all SectionMesh fields, sorting, relevant scratch/light
data and repeated use after fallback. Allocation capacities need not match.

Positive material checks cover independent animated texture layers, mixed face
materials, tint, Top reflection, opaque UV bounds and a real Fusion full-layout
smoke case. They do not cover every Fusion predicate, layout or animation shape.

## CPU measurements, not optimized-game performance

Each result is the median of seven samples, with eight warmup calls and 64 calls
per sample, alternating baseline/adapter order. The profile is opt-level=0,
debug=0, incremental=0, with debug assertions. Each timed section contains 256
separated cells; the `mixed` labels mean custom cells amid air, not stone/custom
mixing. These are section-meshing times, not frame rates.

| Case | Native median | Adapter median | Ratio |
| --- | ---: | ---: | ---: |
| No provider | 4.739 ms | 4.769 ms | 1.0064 |
| Registered, ordinary stone cells | 4.742 ms | 4.785 ms | 1.0092 |
| Provider declines the custom cells | 4.802 ms | 4.956 ms | 1.0321 |
| Claims 256 cells with custom geometry | 4.797 ms | 9.045 ms | 1.8855 |
| Claims 256 cells with secondary material | 4.781 ms | 9.211 ms | 1.9265 |

The claimed paths add about 89% and 93% CPU time in this unoptimized synthetic
workload. This is not a no-performance-cost result. Small no-provider/ordinary
differences are near sampling noise, not an optimization win. Optimized release
measurements and real-game profiling remain necessary before broader performance
claims.

## Preserved failure and evidence limits

The first run at `1f7fddcb` failed. Its negative input used element `shade: false`,
which the pinned ordinary cuboid decoder ignores. The corrected input uses the
decoded `shade_direction_override: "north"`, and asserts the actual baked source
None/template Some(North) before the original unchanged fallback comparisons.
Production code and the full native oracle were not changed. The original false
receipt and partial timings remain in
`experiments/terrain-material-cpu-ci/first-attempt.json`.

The complete decoded compact job console was recovered: 78,798 UTF-8 bytes,
SHA256 `4a746fa27f7b89ba01cd1248abf20d440c58e334a2b24c843dc6d5f798784e21`.
The companion JSON preserves exact named results, every phase's raw-log digest,
compiler invocation hashes, samples and the strict final receipt. Raw per-phase
compiler logs were verified inside the runner but not uploaded; their hashes do
not provide access to those logs.

**Still unverified:** GPU display, production resource packs, live gameplay,
TConstruct bootstrap/provider/model registration, all 70 fluid families in a live
world, fluid physics/immersion/network behavior, optimized performance and
exhaustive resource-pack/Fusion compatibility. This CPU success does not complete
those independent tasks.
