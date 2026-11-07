# Working agreement for horizon-mods

Agents (Codex, Claude Code, others) and humans follow this file. `CLAUDE.md` points here.

## What this repository is
Tinkers' Construct for Horizon (`tconstruct/`) built on PlutoLib (`pluto-lib/`), a library that
absorbs the differences between Horizon and Horizon Mixins. Upstream is the official
`horizon-rust/horizon` and `horizon-rust/horizon-mixins` (default branches). We do **not** control
either and never edit them. The owner's old personal fork of horizon is not a dependency.

## Hard rules
1. **PlutoLib only.** TConstruct code uses PlutoLib APIs. It must not call the fork, Horizon internals
   or Mixins internals directly. `python3 tools/lint_forbidden_api.py` enforces this in CI: fork
   references and non-official git dependencies are always errors; legacy host references are frozen by
   `tools/forbidden_api_baseline.json` (they may shrink, never grow). If PlutoLib lacks an API, add it
   to PlutoLib in its own PR, then use it.
2. **One PR = one verification unit.** A PR changes one feature or one repair, and states exactly what
   was run and what was not.
3. **At most 5 open PRs at a time.** Merge or close before opening more. Do not stack PRs on PRs.
4. **Local smoke test is required** for any change that touches game behaviour: run
   `tools/smoke-local` (see below) on the Windows/Linux machine that has the game resources, and paste
   the `SMOKE PASS` output. CI cannot launch the game.
5. **Say "verified" only for what ran.** "Builds", "launches", "mod loads" and "feature works" are
   different claims. Anything not executed is written as "unverified". Never write "started OK" unless
   the game actually started with the mod loaded.
6. **No new features during recovery.** The goal is the foundation and re-absorbing existing work
   (see `docs/migration/` when present).
7. **Never bypass a PlutoLib guard** (`pluto-lib/tools/check_*_plan.py`). They hash-pin official source
   on purpose. A guard that fails after an upstream bump means that feature needs re-review.

## Upstream pins
`tconstruct/compatibility.json` is the single source of the pinned Horizon and Mixins revisions
(PlutoLib/TConstruct manifests carry the same values). Use `python3 tools/bump_upstream.py --horizon SHA --mixins SHA`
to change them, then rebuild with the slice pipeline. `.github/workflows/upstream-drift.yml` builds
against upstream HEAD weekly to detect breaking changes early.

## The vertical slice
Upstream moves fast and rejects some hook targets, so only a slice is built and launched:
`tools/smoke-local/slice.json` lists what stays enabled; `tools/slice_manifest.py` generates reduced
Mixins bundles from the full manifests (which stay untouched) and `stage.py --pluto-guards` runs only the
PlutoLib guards of kept features. Everything else is **parked, not deleted**: restore a feature by adding
its injection id to `slice.json`, then fix whatever the plan/guards/compile report. Do not edit the full
manifests just to make the build pass.
CI steps that need the full manifest are kept in `.github/parked/mixins-full-checks.yml` (not a workflow) and move back into `mixins.yml` when their feature returns.

## Commands
```sh
python3 tools/lint_forbidden_api.py                 # forbidden-API lint
python3 -m unittest discover -s tests               # repository tool tests
# CI-equivalent (no game): stage the slice and cargo check
python3 tools/smoke-local/build.py --ci --horizon ../horizon-official --mixins ../horizon-mixins
# Local gate: build, prepare resources, launch, assert (needs a prepared Horizon 26.3 resource folder)
python3 tools/smoke-local/build.py --horizon DIR --mixins DIR --base-resources RESOURCES
python3 tools/smoke-local/smoke.py --game CLIENT --ctl CTL --resources game-resources
```
Upstream checkouts must sit at the pinned revisions (`git fetch --depth 1 origin SHA && git checkout FETCH_HEAD`).
If the default temp drive is small, set `TMP`, `TEMP`, `CARGO_HOME` and `CARGO_TARGET_DIR` to a roomy
drive. Building the client takes ~10+ minutes and several GB; do not run it while someone is playing on
the same machine.

## Known state
See `docs/development.md` ("Current status") for what has actually been verified.
