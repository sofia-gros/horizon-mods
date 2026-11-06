# Outer-tank memory diagnostic: expected stop

On 2026-11-06, [run 42183008828](https://github.com/sofia-gros/horizon-mods/actions/runs/42183008828) at [9db4f9a2](https://github.com/sofia-gros/horizon-mods/commit/9db4f9a26ed45f2d3eb79ca243c5d598fb48293a) completed its read-only memory observation and intentionally exited 1. GitHub reports failure because this diagnostic-only checkpoint cannot start setup or compilation. It is **not a native test failure or success**.

The exact final receipt has `setup_started=false`, `native_started=false`, `passed=false`, `verified=false`, and empty `gates`. Only the pinned repository checkout preceded the observation. No additional source fetch, rustup, Cargo, Rust compilation, or outer-tank test ran. The frozen 160-case suite remains **not run**; the earlier separate 146-case result is unchanged.

## Observed facts

- Hosted Ubuntu 24.04.5 runner, image 20261101.360.1.
- MemTotal: 16,366,512 KiB. MemAvailable: 15,411,796 KiB at this observation.
- Self cgroup: `/system.slice/hosted-compute-agent.service`.
- cgroup2 mount: `/sys/fs/cgroup`, reported mount root `/`, with `nsdelegate,memory_recursiveprot`.
- Service and system.slice nodes reported `memory.max=max`; their observed `memory.current` values were 497,414,144 and 1,309,007,872 bytes.
- At the visible root, `cgroup.controllers` was readable, but `memory.max`, `memory.current`, `cgroup.events`, and `cgroup.type` returned ENOENT.

These facts do not by themselves authorize treating an unobserved ancestor limit as unlimited or relaxing the guard. A separate reviewed source cutoff is required before enabling any native path. No retry was launched. The prior unconditional diagnostic stop remains in force.

## Reproducible evidence and scope

[The JSON receipt](outer-tank-memory-diagnostic-20261106.json) retains the exact observation, final false receipt, source parent/tree, seven published blob identities/modes, job/run IDs, and UTF-8 job-log hash. The seven remote files were read back byte-for-byte before the one branch creation/push. Local source `391019eb8ad0e7d3f34b8c03b15b94515f583c82` passed 17 pure Python control/source regressions. Those checks prove the diagnostic gate and source rules, not native semantics.

This docs-only closure uses a separate branch so it does not rerun the diagnostic-only workflow. No production code, native source, API, existing PR state, local dependency cache, or user fork was changed or inspected.

