# Roadmap

Owner-defined order, with the engineering adjustments agreed in review. Do not reorder without the owner's decision.
Principles: [PlutoLib design principles](../pluto-lib/docs/design-principles.md).

| # | Step | Done when |
|---|---|---|
| 1 | **Fix the PlutoLib specification.** Fabric-API-like modules, names and semantics; independent mod packaging; one release per Horizon version; Tinkers-needed scope first. | A reviewed spec lists the modules, their Fabric API counterparts and the Rust naming rules, and the owner approved it. |
| 2 | **Find the APIs PlutoLib needs**, from the current `main` hooks and the open PRs (`pluto-lib/docs/fabric-api-hook-map.md`), checked against the original Tinkers' Construct OSS source for shape. | Every legacy hook and every API the open PRs add is mapped to a PlutoLib module or marked mod-specific. |
| 3 | **Build the APIs.** Module by module, each with tests and a local smoke assertion; port PlutoLib to the current Horizon (re-review the guarded seams). | PlutoLib builds as its own mod, passes its guards on the pinned Horizon, and covers the modules from step 2 in the agreed order. |
| 4 | **Move today's Tinkers `main` onto PlutoLib** feature group by feature group, removing the direct injections. | `tools/forbidden_api_baseline.json` is empty for `tconstruct/`, CI and `tools/smoke-local` are green. |
| 5 | **Rewrite the open PRs onto PlutoLib while merging them**, in the order of `docs/migration/pr-classification.md`. | Each PR: CI green + smoke, originals closed with a pointer. |
| 6 | **Clear every PR and issue.** | No open PR or issue is left unexplained: merged, closed as superseded, or deliberately deferred with a reason. |
| 7 | **Release.** | Release criteria below are met. |
| 8 | **Inventory what the original Tinkers' Construct has and `main` lacks.** | A tracked list of missing features, by feature group. |
| 9 | **Deliver them as updates**, on PlutoLib. | One release per feature batch. |

## Adjustments to the order

- **Steps 1 to 4 are iterated in thin vertical slices, not run as waterfalls.** Specify one module, build it, move one Tinkers feature group onto it, run the smoke test, then take the next. Slices follow **Fabric API modules** (for example lifecycle and tick events, item and block settings, interaction callbacks, screen handlers), never one Tinkers feature: a Tinkers feature is only the first consumer used to test a module. PlutoLib never gets an API that exists for a single feature such as the Part Builder. Late integration would only reveal API mistakes at step 4.
- **Packaging and versioning belong in steps 1 and 3**, before step 4: PlutoLib as its own mod, a version scheme tied to the Horizon revision, and a compatibility table.
- **Step 8's inventory is also used lightly in step 2** so the API shapes do not have to be redone when the missing features arrive.
- **A Mixins limit never removes an API.** Where the pinned Horizon Mixins cannot express a hook yet (today: mining speed, Tinkers Chest pick color), PlutoLib still provides the full Fabric-API-shaped call; its body is a stub (a no-op, or `false` / the default value) until Mixins allows the real implementation. Mods are written once against the API and never change when the stub is filled in. A support matrix in the PlutoLib docs lists every stubbed call and why.
- **During steps 1 to 3 the tree is frozen:** no new features, no new direct injections (lint ratchet), and no mass PR merging that would have to be rewritten at step 5. Existing legacy hooks stay as the working baseline for smoke tests.

## Release criteria (step 7)

CI green on the release commit; `tools/smoke-local` passes against the pinned Horizon; the release is built from a pinned PlutoLib/Horizon/Mixins triple that is written down; save/reload and LAN paths for the shipped feature groups are exercised, not assumed; README and docs state what is and is not supported.
