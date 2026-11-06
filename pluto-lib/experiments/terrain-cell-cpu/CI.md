# Manual standard-runner CPU experiment

This WIP mode extends the existing `Mixins validation` workflow, which already
exists on the default branch with `workflow_dispatch`. It does not change the
normal validation body or run automatically. Select this checkpoint branch and
set `terrain_cpu=true`; the dedicated CPU job runs. Select `game_borrow=true`
as well to request the independent compile-only Game job in the same dispatch.
The broad consumer job runs only when neither mode is selected. No separate
release/tag or main merge is required.

The job uses the existing standard `ubuntu-24.04` runner class, `contents: read`,
checkout with credentials not persisted, and no supplied secrets. It does not
write repository state, change account/app permissions, enable billing, use a
larger/self-hosted runner, save/restore Actions caches, upload artifacts or create
custom images. Results stay in workflow logs and the job summary. A dispatch
requires existing repository write access; no new token or OAuth grant should
be created merely to run this experiment.

On 2026-11-06, the repository API reported `sofia-gros/horizon-mods` as public.
[GitHub's billing documentation](https://docs.github.com/en/billing/concepts/product-billing/github-actions)
states that public standard-runner compute is free, and logs/job summaries do
not count toward artifact storage. Larger runners and additional stored outputs
are deliberately excluded. The script checks public visibility again from the
dispatch event and rejects a private or foreign repository. No account billing
dashboard was inspected; this conclusion applies only to this bounded mode.

[GitHub's runner specification](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
lists 4 CPUs, 16 GB RAM and 14 GB SSD for this public Linux runner class. That is
not proof of available build space. Every expensive subprocess, including source
fetch/export and toolchain installation, is monitored at 250 ms intervals; it
stops if free space drops below **10 GiB + 128 MiB**. There is no fallback to a
paid larger runner or a weaker floor. It uses two build jobs, no incremental
codegen and no debug info. A clean runner still may fail the capacity gate.

## Reproducible execution

1. Verify the dispatch commit equals the checked-out commit.
2. Fetch only official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and
   official Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`. Export their exact
   committed source with the existing safe archive reader.
3. Build the pinned Mixins CLI, package the exact ordinary v15 library, and
   require archive SHA-256
   `7d9c4c5d7bd91ec6584b9206658f9d5bdb5d04dd3f95f1cfe90ceca6daae5386`.
4. Add the one test-only fixture module, run the real complete plan, all twelve
   production composition guards and the fixture-only identity guard, then apply.
5. Compile and execute the actual renderer's one exact unit test in separate
   no-provider and registered processes. The first invocation obtains only the
   selected target's locked dependencies; the second runs offline. No unrelated
   unit tests execute, though the full native renderer test crate is compiled.
6. Require a non-skipping one-test pass and all expected timing records. Emit
   raw seven-sample timings, the actual source identities, minimum free space
   and terminal status in logs/summary. Hash every applied source file, including
   Cargo.lock, before and after both runs; re-verify all original fixture inputs
   and unchanged production payloads afterward. Only unchanged-source success is
   accepted. A failed or disk-stopped run is not proof.

The measurements are unoptimized CPU diagnostics using original synthetic
resources. They are not production-frame performance, production asset-pack,
TConstruct activation, multiplayer, GPU upload or visible GPU rendering proof.

This combined checkpoint is distinct from the unexecuted v14 CPU-only branch
at `74d4cb7c`, which remains unchanged. The production renderer adapter still
has the exact frozen `d63a7c8e` bytes. Moving to v15 requires a new actual run;
neither ordinary typechecking nor the old source tests count as a CPU result.

An existing authorized CLI can request both independent jobs with:

    gh workflow run mixins.yml --repo sofia-gros/horizon-mods \
      --ref checkpoint/plutolib-bounded-native-ci-20261106 \
      -f terrain_cpu=true -f game_borrow=true

Omit either true input to request only the other mode. The Game job's exact
positive/negative contract and limitations are in `../game-borrow-ci/README.md`.

Alternatively select the same branch in the existing workflow's Run workflow
form. [GitHub documents branch-specific manual dispatch](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow).
Publication and actual dispatch are recorded separately; this file alone does
not establish that a run started or passed.
