# Latest terrain reload: excluded source candidate

Target Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`, Mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57`. This candidate is not installed,
compiled, planned, applied or executed. It adds no runtime cache or registry.
The successful 624c CPU fixture and unexecuted old profile remain unchanged.

## Current implementation decision

No cache runtime defect has been demonstrated. Latest `stage_pack` reuses the
existing Assets Arc but constructs a new RenderAssets Arc. `PackResources::load`
then constructs a new MeshTables Arc and enters the existing assembly HEAD.
That gives a distinct owner triple even for the same root, equal model values
or identical diagnostic generation. `context_with` only changes biome context
and preserves the same three owners. The current weak-owner cache already
expresses this lifetime; a new global generation/active-world map is unnecessary.

Successful `apply_staged_pack` swaps the pack before supplying `worker_context`
to `Streaming::reload_resources`. Native streaming drops old in-flight results.
Our cache may still read facts for retained old owners; this is not permission
to publish an old mesh after the native swap. A discarded staged candidate may
have prepared a weak cache entry without becoming current. It must not replace
the old owner's entry or keep its native resources alive.

The new live Mod switch path eventually uses this same resource assembly.
Unsupported registration changes still make old revisions unavailable and need
new resource owners. This audit does not make live block/item registration or
TConstruct consumer activation available, and does not repair the separate
slime-colormap `load_with`/`load_with_parsed` coverage concern.

## Candidate proof layers

`source_contract.json` pins complete latest lifecycle source and the exact
current cache/projection bodies. `source.py` checks those bytes and scopes the
reviewed call-order evidence; it is not a planner or a Rust AST interpreter.
These checks do not replace complete installed-plan guards. Existing production
guards are unchanged pending the first actual latest composition diagnostic.

The small Python check passed over five selected latest official files and
seven historical production/fixture files. It checked lexical call order, the
one-test/six-case source inventory and exact removal of the proposed insertion.
The composed source hash was
`a68aa7c3a7af27439e0bc213ef620a45c44e09a8309a80940b4b7695f26bdacc`.
No composed file, official checkout copy or native executable was produced.
These selected-file checks do not prove complete source ancestry or Rust typing.

`native.rs` is intended as a nested extension to the frozen successful
material-aware CPU fixture, not as a standalone replacement for Horizon types.
It contains one unrun test with six named cases:

1. Same Assets with new RenderAssets/MeshTables, colliding generation, and
   old→new→old observation while both owner sets are retained.
2. Arc clones match; mixed owner triples remain unbound.
3. An unavailable new candidate cannot replace an old ready catalog.
4. An abandoned prepared candidate does not keep native resource values alive.
5. Stale registry inspection stays unavailable on repeated same-owner prepare;
   a new owner triple can bind the new revision.
6. A conflicting same-owner snapshot rejects without replacing the original.

This uses actual Assets, StateModels/Sprites and MeshTables through the existing
original synthetic fixture. Its RenderAssets is assembled from actual baker
output, not the complete native RenderAssets loader. The owner transition is
deliberately constructed; neither `stage_pack`, Game, GPU resources nor actual
asynchronous job cancellation executes. The test does not measure performance.

## Before execution

First preserve the latest main/common one-shot diagnostic. Then choose a new
reviewed terrain package and new plan-derived identities, retaining original
body regeneration, whole-mesher exclusivity and full lifecycle source pins.
The old `terrain-material-cpu/prepare.py` intentionally rejects f4dc; it must not
be called with a spoofed old revision marker. A separate latest preparer must
verify complete source/lock/package identity, include this exact extension with
an inverse-removal check, and keep the original full-body oracle. Only after
ordinary latest apply/type success should the renderer test be run under a
separate reviewed serial CI. New CPU/profile evidence remains distinct from
GPU and live-game behavior.

The first latest diagnostic has since timed out at 180.231 seconds with no
planner output; it did not establish compatibility or reach the old guards.
The separately authorized bounded observation is independent of this candidate.
No guard pins have been advanced on the strength of either source inspection
or this unexecuted test.
