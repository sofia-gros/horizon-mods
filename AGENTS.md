# Working agreement

For every agent (Codex, Claude Code, others) and human working here. `CLAUDE.md` points to this file.

## What this repository is

Two products in one repository: **PlutoLib** (`pluto-lib/`), Horizon's Fabric API, and **Tinkers' Construct for Horizon** (`tconstruct/`).
PlutoLib is an independent, general-purpose mod that follows Fabric API's names, features and behaviour (Horizon Mixins is expected to offer the NeoForge-like API),
is released once per Horizon version, and lets mods (Tinkers now, anyone's later) survive Horizon updates with little or no change.
Read [pluto-lib/docs/design-principles.md](pluto-lib/docs/design-principles.md) before changing PlutoLib or adding a hook; the owner decides changes of direction.
Upstream is the official `horizon-rust/horizon` and `horizon-rust/horizon-mixins`; we never edit them, and the owner's old personal fork of Horizon is not a dependency.

Where things are: [README.md](README.md), [docs/architecture.md](docs/architecture.md), [docs/development.md](docs/development.md) (build and checks), [docs/releasing.md](docs/releasing.md), [docs/roadmap.md](docs/roadmap.md) (the plan, the permissions and the decision log).

## Rules

1. **PlutoLib only.** Mods call PlutoLib (Fabric-API-shaped); if an API is missing, add it to PlutoLib first. PlutoLib's API is always complete: where the host cannot do something yet, the call exists as a documented stub, never missing. No API is designed around a single feature of a single mod.
   `tools/lint_forbidden_api.py` rejects fork references and non-official git dependencies; the legacy direct injections in `tconstruct/` are frozen by `tools/forbidden_api_baseline.json` (may shrink, never grow).
2. **One pull request, one verification unit.** It says exactly what was run and what was not.
3. **Verify locally first.** Run the local gate in [docs/development.md](docs/development.md); CI is a slower second check, not a precondition. Anything that touches game behaviour also needs `tools/smoke-local`.
4. **Say "verified" only for what ran.** Builds, launches, mod loads and "feature works" are different claims; unexecuted means "unverified".
4c. **Scope and ownership rules from the owner.** (1) No mod-integration items, blocks or systems are written (the original's `plugin` package is out of scope). (2) PlutoLib's responsibilities and specification (`pluto-lib/docs/design-principles.md`) are re-read before PlutoLib work and never bent for one mod. (3) PlutoLib keeps, and builds on, the creative-tab feature of the old bundle: more tabs than fit are paged, and a mod's tab items can be taken out of it (`pluto-lib/mixins/pluto_creative`, `pluto_native_creative`). (4) Mods built on PlutoLib never compete for injections: PlutoLib owns each host injection point once and dispatches to registered callbacks in a defined order; a mod ships no injection of its own (`tools/check_injection_ownership.py` fails on a second PlutoLib hook of one place, a mod hooking a place PlutoLib owns, or a mod gaining injections beyond `tools/injection_limits.json`).
4b. **The original Java is the specification.** The open-source Tinkers' Construct at the revision pinned in `tconstruct/compatibility.json` is what the port must match; the existing Rust is of unknown correctness. Port a whole domain at a time (for example the whole smeltery) from the Java onto PlutoLib instead of moving code injection by injection; a domain is one design, one build and one in-game check. A shallow clone for reading lives in `.local/tconstruct-source`.
4a. **PlutoLib API changes carry their documentation.** Adding or changing a public item means writing or updating its section (types, arguments, a working example for every form) in `pluto-lib/docs/api/`; `tools/check_api_docs.py` and the doctests fail otherwise.
5. **No new features while PlutoLib is built**, and no new direct injections.
6. **Keep tests few and real.** A test must fail when the behaviour it names is broken. Never write tests that assert source or document text, copy upstream files, or exist only to pass.
7. **Line endings are LF everywhere.** `.gitattributes` forces it whatever `core.autocrlf` says; `python tools/check_line_endings.py` checks it. In Python write files with `newline="\n"`.
8. **Never bypass a PlutoLib guard** (`pluto-lib/tools/check_*_plan.py`); a failing guard means that feature needs re-review.
9. **Documents stay few and current.** No per-implementation notes; detail lives in code (rustdoc, tests). Update the existing documents instead of adding new ones.
10. **Releases** only by tag, per product (`pluto-lib-v…`, `tconstruct-v…`), and only when [docs/releasing.md](docs/releasing.md)'s criteria hold.

## Pins and the slice

`tconstruct/compatibility.json` pins the Horizon and Mixins commits; `python tools/bump_upstream.py --horizon SHA --mixins SHA` changes them everywhere.
Upstream moves fast, so a *slice* is built and launched (`tools/smoke-local/slice.json`): everything the pinned Mixins accepts, parked rather than deleted otherwise. Do not edit the full manifests to make a build pass.

## Habits that avoid known trouble

- Write commit messages and PR bodies to files (Write tool), then `git commit -F` and `gh pr create --body-file`; shell heredocs mangle backticks and apostrophes.
- Windows GUI programs (the launcher, Horizon) print nothing unless their output is redirected to a file.
- Builds are large: put `TMP`, `CARGO_HOME` and `CARGO_TARGET_DIR` on a roomy drive and keep 10 GB free.
