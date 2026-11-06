# Terrain cost-isolation source candidate

**Unexecuted Rust source.** There is no new workflow, native compile, apply,
renderer execution or timing result in this checkpoint. The parent design is
`docs/terrain-cpu-profile-plan-20261106.md`; that checkpoint remains the original
design record. This source candidate does not revise any earlier measurement.

`profile.rs` is one excluded nested test module. `source.py` can compose it only
in memory, immediately before the exact native baseline marker in successful
fixture `1ac143d8`. Removing the delimited insertion must reproduce every byte
of that original generated fixture. The official function, original two modes,
all material assertions and all existing fallback/replay tests remain intact.
Production `43f17427` and bundle `cd6de18c` are not edited.

One prospective exact test is
`mesher::pluto_terrain_material_cpu::tests::profile::same_output_callbacks_and_microkernels`.
It must eventually run in its own process, separate from the two original CPU
modes, so its registry does not change their workload. It admits only the new
`profile` mode and external `opt0-assertions` / `opt3-assertions` labels. Those
labels alone are not compiler evidence; the future runner must verify actual
flags and Cargo artifacts as specified in the design.

## Controlled comparisons

Seven groups are registered before the first mesh: five measured callbacks and
two verification-only callbacks. The latter receive the real native adapter
context and compare direct original/build/clone batches and material slots.
The fixed geometry is initialized once before any registration or timing. It
uses six front faces, half height, full UVs and no backfaces. The fixed callbacks
explicitly decline other variants and the Top material orientation. This is a
fixture-only immutable constant, not a cache proposal for arbitrary Mods.

Twenty explicit preflight rows cover three geometry comparisons and two material
comparisons in four lighting combinations. Each compares five reused sections
(`single`, `mixed`, `ordinary`, `empty`, `single`), complete unsorted/sorted
outputs and the original scratch/light/leaf observations. Half-height and
256-cell vertex-count assertions reject accidentally comparing native fallback
with itself. Each timed pair additionally verifies these exact owners before
starting clocks. Three unsupported constant contexts must retain native output
and replay. No test-only check callback is used in a timing loop.

Three paired rows compare original geometry callback versus lean construction,
lean construction versus fixed clone, and original material construction versus
fixed clone with the same secondary material. Eight warmups, 64 iterations and
seven alternating samples are fixed. All sample totals are printed. Five pure
microkernels use the actual production build/clone/cell-local/UV-subset functions,
black-boxed inputs/results, 128 warmups and 4,096 iterations in seven rotating
rounds. They are not subtracted from meshing time to invent an emission cost.

An observed minimum empty-clock interval is diagnostic, not guaranteed clock
resolution. Samples shorter than one millisecond or 100 observed intervals are
explicitly marked inconclusive. There is no speed threshold or favorable-result
auto-tuning. Actual compiler settings, binary identity, all raw case receipts,
resource/lock/source seals and the original end-to-end modes still require the
separate future runner and serial release.

Ten source-only Python checks passed on 2026-11-06. They check pins, exact insertion/inverse, native oracle
preservation and bounded source contracts. They cannot type-check this Rust or
prove equivalence/performance. The strict extended-overlay verifier and planner
guard are not implemented yet. Do not call the old verifier on changed source
and ignore its failure. Do not run this source locally, infer GPU/live-game
results, or claim that TConstruct startup/model registration has been connected.
