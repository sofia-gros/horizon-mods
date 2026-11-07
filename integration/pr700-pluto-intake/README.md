# PR #700: use the existing Pluto potion catalog boundary

Selected PR: https://github.com/sofia-gros/horizon-mods/pull/700
Work branch: `feat/casting-tank-tagged-379`, starting at `bf0333ea11188a4009a2ed5cf25965fdcb88c575`.
This is a source migration, not a successful native build or a main merge.

## Shared responsibility and consumer policy

The ordinary Pluto `pluto_potions::potion` adapter already reads the official native catalog. Its complete 2,669-byte source and existing public module declaration are reused unchanged from `9e032ef5b013487ea08b997eb08df84825fb09ba`. No new API, registry, cache or potion list is introduced. The original PR branch did not contain that adapter, so this branch receives the existing source and declaration. The current integration already contains them and must not duplicate the module.

TConstruct retains its own admission policy: only its potion fluid, one exact base-potion tag, Minecraft namespace, a registered potion and its supported fluid name. Custom colors/effects, unknown potion IDs and other tagged forms continue to wait. The adapter establishes catalog facts, not permission to move fluid.

The original PR's source and recipe rechecks, source-container reprocessing, exact potion identity, staging, combined save, notifications, packet amount and failure behavior are preserved. The branch runtime differs from the original PR by only its catalog function path. The five original runtime edit spans are separately ported onto the current integration source without replacing other Pluto migrations.

## Tests and boundaries

- Eight Python source/inverse checks passed. They inspect real candidate files and do not emulate a game or prove Rust typing.
- The original eleven Rust route cases remain. One additional case compares the actual shared adapter with the native catalog and checks that TC policy remains separate.
- All twelve Rust route cases are **not run** in this revision. No Cargo, metadata, native compiler, CI, apply, world or LAN execution was performed.
- The route fixture retains its explicitly substituted world, storage, recipe-selection and networking environment. Real state codecs and the real Pluto catalog adapter are included. Its first 90 mB packet assertions are not proof of completing a 250 mB recipe.
- The historical work branch's compatibility declarations are unchanged. No old engine fork was accessed. Current integration testing must use its approved official cutoff; do not treat the old branch as a current standalone host build.

Source test command:

`PYTHONDONTWRITEBYTECODE=1 python -B -m unittest discover -s tconstruct/tests -p test_faucet_potion_pluto_migration.py -v`

## Exact current-integration intake

`intake.json` records four exact before/after files against `9e032`: runtime, route fixture, locked fixture runner and the original PR documentation. `apply.py::build(root)` validates that source and existing Pluto adapter, then returns the four postimages in memory. It does not write a worktree or move a Git ref. The actual four postimages were generated and verified against this fixed base.

The current fixture's Pluto storage/potion includes and locked dependency helpers are retained. Its explicit run list is extended to all twelve route cases. `source-melter-state.rs` is not an intake path: the newer `13b4bdb0` storage migration must remain unchanged.

The source check above is specifically for the original work-branch postimages. It is not advertised as an ordinary-integration discovery test; the central intake uses the exact transform and the current locked fixture runner instead.

## Whole original PR and lifecycle

The original PR owns five files. Runtime, fixture, runner and documentation have concrete current-integration postimages here. The original workflow invocation remains unchanged on the original work branch. Its current-integration CI scheduling is still a build-owner task; it must not be silently marked included merely because the runner exists.

All three workflow triggers on this work branch and current main were checked before publication: Mixins is manual/reusable, release is the `v0.5.0` tag, and the only pull-request workflow is path-filtered to the unrelated config parser. This source push does not select a native job. No workflow is weakened or disabled.

Status: `changed-pending`. Main is written only through the coordinated integration path after the required current build, no-error and startup conditions. Close #700 only after its complete own diff and necessary tests are accounted for in main. This work does not complete all fluid routes or the seventy-PR storage partition.
