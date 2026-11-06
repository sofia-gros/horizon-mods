# Tinkers latest playtest integration

**Status: source integration in progress. This branch is not yet a runnable release.**

This is the single review branch requested for trying the latest Tinkers work. Main and the existing PRs are unchanged. The intended target is official Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf`, official Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`, and Windows 11 x64.

## Included now

- PR #944's shared foundation and TConstruct migrations, plus PR #945's navigation changes.
- Later ordinary library and consumer changes from common `035273c4`.
- Exact, conflict-free three-way Git-blob composition against `bcec17b5`: 6,749 source files. Modes and contents are preserved. No test result is inferred from that source merge.

The original compatibility declarations remain as source evidence at this first checkpoint. They are **not** a claim that the combined branch supports the intended latest target. Reviewed latest adapters and pin changes are still being assembled.

## Still being integrated

The exact source inventory and pending reasons are in [integration/playtest-20261106.json](integration/playtest-20261106.json). Guide lazy-text/BookId changes need two overlapping consumer files merged without losing PR #945 navigation. Mining/chest compatibility changes must be rebased to this composition rather than the previous main-only TConstruct input. Guide LAN and crafting composition boundaries still need integration. Latest source guards remain mandatory.

Retained Guide storage, item-name/default ownership, locale repair/root-key, terrain optimization/profile and unexecuted Font generation experiments are **not enabled** by this merge. Their source checkpoints remain distinct until the relevant integration and runtime gates pass. No feature has been silently removed to obtain a pass.

## Build and launch status

- Complete plan for this branch: not run.
- Mandatory composition guards and applied native build: not passed for this branch.
- Windows executable/link/DLL verification: not run.
- Game window, GPU, save/reload and LAN: not verified.

The most recent prior full probe used a different TConstruct composition and Horizon `f4dcf8a2`; it stopped at the chest-pick hook. Its [retained diagnostic](https://github.com/sofia-gros/horizon-mods/commit/5db8371b1018355c9686e5e801c76900cabd5750) is not proof that this branch builds.

**Do not treat the existing legacy launch scripts or a packaged .hnmod as a verified latest playtest.** There is no verified Windows binary or safe branch-specific launch command yet. The next steps are complete plan, all mandatory guards, apply/native build, Windows link/package checks, and a bounded launch check where an appropriate machine is available. Exact commands and artifact hashes will be added here when those gates pass.

Minecraft assets, Microsoft login information and saved worlds are not part of a published build. A future playtest should use a separate instance and a new test world or a copy of a backed-up save. Official launcher asset preparation remains on the user's machine.

At least 10 GiB free space and the stronger existing execution guards are retained. No Horizon fork is used.
