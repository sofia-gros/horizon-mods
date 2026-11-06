# Manual actual Game borrow proof

This is an **unexecuted CI checkpoint**, separate from the terrain CPU fixture
and from the ordinary v15 package/type proof. It installs no production hook,
applies no Guide storage proposal and runs no game, guest, renderer or GPU.

The existing `Mixins validation` workflow has two independent, opt-in booleans:
`terrain_cpu` and `game_borrow`. Select either or both in one manual dispatch.
The broad consumer job runs only when neither is selected. Each selected mode
uses its own standard public-repository `ubuntu-24.04` runner, `contents: read`,
checkout without persisted credentials, no cache/artifact action, no secrets,
no write token and no larger paid runner. No permission or billing setting is
changed. Ordinary local Cargo download/build files exist only on that runner.

Both modes use official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`,
official Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`, Rust 1.95.0 and exact
v15 package SHA-256
`7d9c4c5d7bd91ec6584b9206658f9d5bdb5d04dd3f95f1cfe90ceca6daae5386`.
The previous CPU-only checkpoint at `74d4cb7c` remains unchanged and targets
v14; this new combined checkpoint does not inherit an unrun CPU result from it.
Production terrain adapter bytes remain the frozen `d63a7c8e` implementation.

## Exact compile-only gate

1. Verify the immutable dispatch checkout, fetch only the two official pinned
   repositories, build the official locked Mixins CLI, and package exact v15.
2. Plan all ordinary library sources, run all 12 production guards, then apply.
   Seal all 5,047 resulting files and require the reviewed canonical v15 seal,
   including the actual Game and Cargo.lock. No test hook is installed.
3. Append the exact positive probe from source checkpoint `22c62f30` (published
   `6e44c9bb`). Check the real `horizon-client --lib --no-default-features` with
   Cargo, locked dependencies, one job, no debug info or incremental output.
   The first run may fetch locked dependencies. It must exit zero and log one
   actual host-library rustc invocation with its native generated OUT_DIR.
4. Replace only that appended text with the frozen negative probe. Run the same
   target offline/locked. Require exit 101 and exactly one coded compiler error:
   E0499, with the primary source span on the added
   `game.add_player_effect(effect)` call. At most one exact spanless, code-null
   `aborting due to 1 previous error` summary is tolerated; other errors and
   duplicate summaries are rejected. That optional diagnostic shape has not
   yet been observed in this actual target. Arbitrary compilation failure, a wrong
   target/span, or a skipped/fresh-only host compile does not count as success.
5. Verify the full source/file/lock map after each compile. Native generated
   OUT_DIR contents created by the first compile must not change in the second.
   Restore only the intentional Game addition and verify the complete original
   applied-stage seal again. Emit separate job logs and summary results.

The runner enforces a continuous 10 GiB +128 MiB free-disk floor and terminates
only its own subprocess group on failure. GitHub's job timeout is 60 minutes.
This uses a fresh supported CI runner, not the blocked local hidden-cgroup
environment and not a raw rustc reconstruction of a cached dependency closure.

A pass would prove sequential reborrowing is accepted, while holding the real
MotionWorld player borrow across the whole-Game call is rejected. The positive
probe's uncalled guest/teleport helper is typechecked only. No same-actor/body
identity, continuation authority, live contact handling, ownership, UI, CPU
meshing performance or GPU behavior follows from these compile probes.

The separate guest continuation observer and Guide prototype are excluded.
Small local Python tests check admission, source identity, diagnostic selection,
sealing and workflow constraints only; they are not native compilation results.
