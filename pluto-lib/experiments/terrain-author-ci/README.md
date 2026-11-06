# Terrain author facade: separate native CI candidate

This candidate uses successful material CI head `e210b8c162496b83fec3adca7099ea10cf49b2e8` and only the appended author facade from `2457469595416fe8417b7e34ad2e5dc82a5ffaf7`, plus its four source tests, docs and this dedicated runner. It does not mix in i18n, Declaration/Guide authoring, Java/menu, or any old CPU/Game experiment. The old CI branches and workflows are unchanged.

The production archive is `cd6de18cacbe7c40218ec817671bf3842aef3be0d586c613bb788babafbd3a76`. Its file set matches the successful prior archive `b448763e...`; the **only** changed archive member is existing `pluto_native_creative/rendering.rs`, with the appended thin facade. Manifest entries, lower implementation, guards and authoritative registry remain unchanged. This is a new source cutoff; prior native success is not substituted for any new gate.

## Authorization and execution

Publish/trigger only after the preceding pure-authoring CI is terminal and independent source review is complete. The workflow admits only the exact dedicated `checkpoint/plutolib-terrain-author-ci-20261106` push in the public target repository, on standard Ubuntu 24, contents-read only. No cache/artifact service, credential persistence, PR or billing/permission change. Its separate concurrency group may supersede only this branch's own work.

All setup and output filesystems use the previous continuous 10 GiB + 128 MiB floor, with another 2 GiB required before cold setup. Compilation uses jobs 1, debug 0, incremental 0. Processes have bounded own-group shutdown and must confirm the process group no longer exists even after a successful leader exit. Fixed official Horizon `624c4f62` and Mixins `e8cd0aa3` are fetched directly from horizon-rust; no fork source or modeled host substitute is used. Local native execution is still prohibited.

## New-head gates

1. Exactly 23 Python terrain source guards, including four additive-facade guards. Twenty separate runner/parser/source tests passed during preparation and must pass again before CI setup; these are not native tests.
2. The exact existing 30 production std-only tests and 21 real public fluid API tests (separate immediate modules 10/2/9).
3. Exact archive pin, official complete plan, all 12 guards, official apply.
4. Actual offline/locked host library and named binary type checks. Required rustc observations distinguish API, renderer, client-library adapter, host library and host binary.
5. Compile both actual native unit-test targets, `horizon-client-render` and `horizon-client-lib`, with their real dependencies. Each must show its own rustc `--test` invocation. The official client-library source path is `../horizon-client/src/client_lib.rs`; ordinary lib checking or a `Fresh` line is not this proof.
6. Execute exactly the two existing BakedFace material tests, then exactly the two new `pluto_render::terrain::tests::` facade tests. Exact names/counts, zero failures and zero ignored cases are required separately. The latter establish type aliases and invalid-registration delegation without mutating the global provider map, not successful content registration or drawing.
7. MOD/official/package/API/applied-stage file-set and source/manifest/Cargo.lock seals remain exact. Generated native OUT_DIR content is sealed after both unit-target compilations and checked after both execution phases. Both execution commands are verbose and reject any new rustc invocation. A final dirty/untracked Git check supplements the tracked-file seal.

The expected Rust scope is 55 passing tests (30 + 21 + 2 + 2) if every gate completes. A compiler failure, skipped phase or count mismatch remains a failure with completed gates recorded separately. Source/setup and final-verification errors retain a FINAL_RECEIPT with partial gates, error, verified and passed fields. Success also requires the exact complete gate set.

No live loader/resource-owner resolution, actual 70-fluid content registration, mesher fallback replay, CPU mesh differential/performance, gameplay, upward physics, GPU display, or TConstruct consumer activation is included. The facade does not bypass early model preparation or first-mesh registry freezing.
