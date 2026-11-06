# Terrain facade CI: terminal status and remaining evidence

The dedicated [run 42178767849](https://github.com/sofia-gros/horizon-mods/actions/runs/42178767849) and job `127763240766` completed successfully at exact source `43f174273cd5f1b9cf3acd1d26abab8838025510`, tree `800b5cddd4fe5337f3d398710da3032ded897775`. Both run and job APIs were read independently; all job steps, including the proof runner, report success. The final workflow update is 2026-11-06 07:21:39 UTC.

This is separate from the source-only facade checkpoint `24574695` and the prior material proof `e210b8c1`. The new source uses the successful material runtime plus only the appended terrain facade. Local Python packaging comparison found one changed archive member, `rendering.rs`; the expected package is `cd6de18cacbe7c40218ec817671bf3842aef3be0d586c613bb788babafbd3a76`.

The frozen runner requires 20 runner checks, 23 source checks, 30 production std tests, 21 fluid API tests, two actual native BakedFace tests and two actual client-library facade tests. It separately requires official plan, twelve complete-plan guards, apply, actual host library/binary checks and actual renderer/client-library unit-target compilation. Both test execution phases reject recompilation; final source/lock/generated-input seals and the exact mandatory gate set are required for a successful runner exit.

**Detailed logs have not been recovered.** The ordinary job-log connector repeatedly returned `Transport closed`, including an independent coordinator attempt, while run/job status reads succeeded. Therefore the complete FINAL_RECEIPT, individual result lines, phase timings, minimum free disk and applied-stage/generated-input hashes have not been independently inspected. The companion JSON records these as unavailable, rather than copying them from the older material run or inventing them from expectations. The stated test counts above are the enforced runner contract, not a recovered per-test log inventory.

The source and workflow remain frozen. Retry the ordinary read when available; do not rerun a successful expensive job just to replace missing log evidence. This docs-only checkpoint does not trigger the CI branch again.

No live resource loading/reload, full CPU mesh equivalence or performance, GPU display, upward-flow physics, 70-family content activation, or TConstruct consumer migration is established by this status. These limits also apply after the detailed receipt is recovered.
