# Terrain guards: latest official metadata candidate

This is a narrow metadata adaptation to official Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf` / Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`, based on integration `9cc1a01632913f01df1f0bcbb0b74b8bb9e80525`. No Rust payload, hook, manifest, checker program, copied mesher body/hunk, native culling/emission code, or registry/cache behavior changes.

## Actual observation, not guessed identifiers

[Complete plan run 42231588813](https://github.com/sofia-gros/horizon-mods/actions/runs/42231588813), job `127938548202`, proof head `11794d5f90b274caf46e97b44330b41424ad98de`, accepted the full packaged integration plan. The retained plan identity is SHA256 `7139236d6484c9cf5a862d3e8358d60b2b4e0bd3a8c4c0968c6d313735670f8b`, 6,383,021 bytes, 322 hooks / 321 modules. The unchanged bounded collector retained the actual two terrain hook records and six unique modules, source/payload hashes and an empty competitor list. Its observation is preserved in the companion JSON; `compatibility` remains null. The entire plan was ephemeral and is not reconstructed here.

The old assembly guard rejected the changed AST node, and the old cell guard rejected the changed streaming whole-file hash. That actual refusal remains the baseline. Across all12 old guards only2 passed; no apply or native type check ran. This candidate does not turn the old receipt into a success.

## Exactly eight metadata leaves

- Binding metadata: assembly node `[386,2]` becomes the observed `[398,2]`; the module placement fingerprint for `fluid-render-prepare` follows the actual current `game.rs` module; five exact official source hashes follow `game.rs`, `game/mods.rs`, `game/packs.rs`, renderer `assets.rs` and `streaming.rs`.
- Cell metadata: only the exact `streaming.rs` input hash changes.

Both hook function fingerprints, all bindings, the cell node `[76,1]`, priority/cancellation semantics, all helper/module source hashes, renderer module placement fingerprints, the native mesher whole-file/function hashes, all body transformation hunks and face-filter boundaries are unchanged. Inverting those eight leaves reconstructs both original metadata files byte-for-byte. Both checker programs remain byte-for-byte unchanged.

## Owner / reload boundary reviewed

The complete 2,583-byte `PackResources` implementation is unchanged from the independently verified f4dc source, SHA256 `db92017bff8481741595c0b687011a92af2b3f3caa3b451e460621925bca63e7`. At41f, `load_resources` may use `Assets::reload_with_models`, then wraps assets/render in new Arcs, creates the MeshTables Arc and calls `PackResources::assemble`. The preparation hook is at that assembly boundary, not at the bypassed `RenderAssets::load_with` entry. The worker clones a single context, borrows its assets/render/tables together, calls `mesh_reusing`, and retains face visibility only for the same table Arc.

Thus the previously identified load-with-parsed bypass does not bypass the current assembly hook. No additional mutable cache field or identity based only on a numeric generation is introduced. These are source observations, independently reviewed by the resource owner; they are not an executed reload, renderer, CPU differential or GPU proof. The excluded actual-owner reload fixture remains a separate gate.

## Validation and limits

All13 selected official files were rehashed from previously acquired exact official files and matched the actual collector inputs; no new checkout or fork was accessed. The full mesher source still has SHA256 `364bdbb89839984eaa9819b75170a3ccbe5c73476aec965835edd4eb7e280c19`, and the copied function pin remains `5cbeb23de61b6cf8092a436c7eb2cea410fb9fba7b71c5fa08669d30c54b739f`.

Twelve new pure Python checks passed: exact metadata inverse, unchanged checkers, rehydrated selected-IR replay from hash-verified payloads, old-metadata rejection, old node/module rejection, foreign assembly aliases, whole-mesher-file hook exclusion, module ownership/missing/duplicate/source failures, contract/bindings/cancellation/priority/helper failures, every official input drift, unchanged body/face contract and honest observation scope. The existing14 binding/cell source and packaging tests also passed with the candidate metadata supplied in-process; their original files were not changed.

The replay contains only the collector's retained selected records and is explicitly not a full-plan execution. The collector's original empty competitor observation is retained rather than manufacturing unobserved nodes. A new complete-plan guard run against the exact final integration and pristine official source is still required before apply. The rustfmt-backed regeneration check was not run locally; the exact unchanged native body/hunk/input hashes are preserved for that mandatory check. Latest host type validation, Windows build/link, actual CPU reload/differential and GPU/game playability remain unverified. Old624c/e8 runtime and CPU successes are historical evidence only.

For a sparse local review, set `PLUTO_TERRAIN_RUNTIME_ROOT` to the existing `pluto-lib` root with unchanged payload/checker files and run `python pluto-lib/tests/test_terrain_41f_metadata.py -v`. In a complete integration checkout no override is needed.
