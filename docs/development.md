# Development

How to build, check and release in this repository. Everything here has been run on Windows 11; Linux works the same except where noted.

## You need

- Rust (stable; the launcher installs its own), Python 3.11 or newer, Git, and `gh` for pull requests.
- Checkouts of the official repositories at the revisions pinned in [`tconstruct/compatibility.json`](../tconstruct/compatibility.json):
  `horizon-rust/horizon` and `horizon-rust/horizon-mixins`. Fetch one commit:
  `git init DIR && git -C DIR remote add origin URL && git -C DIR fetch --depth 1 origin SHA && git -C DIR checkout FETCH_HEAD`.
- For anything that starts the game: Horizon's prepared 26.3 resource folder (made once by `horizon-launcher prepare` from your own Minecraft install).
- A roomy drive. A client build needs several GB; set `TMP`, `TEMP`, `CARGO_HOME` and `CARGO_TARGET_DIR` to it if the system drive is small.
  Builds here set `CARGO_INCREMENTAL=0` and stop below 10 GB free, because incremental caches grow without bound.

## The local gate (run these before every pull request)

Quick, no game, no network:

```sh
python tools/check_line_endings.py
python tools/check_docs_budget.py                       # markdown files must be on the allow list
python tools/lint_forbidden_api.py
python -m unittest discover -s tests
python -m unittest discover -s pluto-lib/tests          # about 30 s
cargo test --manifest-path pluto-lib/tests/pure/Cargo.toml   # PlutoLib's host-independent part, seconds
```

Tinkers' Python tests: `python -m unittest discover -s tconstruct/tests` (long: several minutes).

With the game (needs the pinned checkouts and prepared resources):

```sh
python tools/smoke-local/build.py --horizon HORIZON_DIR --mixins MIXINS_DIR --base-resources RESOURCES_DIR
python tools/smoke-local/smoke.py --game <built horizon-client> --ctl <built horizon-ctl> --resources <smoke-build>/game-resources [--screenshots DIR]
```

`build.py` stages the slice (below), builds the client, prepares Tinkers' resources. `smoke.py` starts the game and checks what a player would see: a flint pickaxe crafted in survival, and the Part Builder opening. It prints `SMOKE PASS` only if both ran. `--ci` makes `build.py` stop after the stage and `cargo check` (no game, no resources).

A mod that uses only PlutoLib, in a running game (builds PlutoLib and a throwaway consumer with the launcher, starts the game, requires the consumer's log lines):

```sh
python tools/smoke-local/probe_pluto_lib.py --launcher <horizon-launcher> --resources RESOURCES_DIR
```

With `--full <extracted legacy PlutoLib folder>` the probe also builds the whole sliced Tinkers (about 7 minutes with the launcher's `dev` profile; without it the probe takes about 2).

PlutoLib through the official launcher (the way players get mods):

```sh
horizon-launcher install latest && horizon-launcher prepare            # once
horizon-launcher native-build FOLDER --server-output OUT --trust-native-code
```

On Windows the launcher is a GUI program: start it with its output redirected to a file (`Start-Process -RedirectStandardOutput`).

## Pins, slice and guards

- Horizon and Mixins are pinned by exact commit. `python tools/bump_upstream.py --horizon SHA --mixins SHA` rewrites every pin; then rebuild.
  A weekly workflow (`upstream-drift`) checks the newest upstream.
- Upstream moves fast, so a **slice** is built: [`tools/smoke-local/slice.json`](../tools/smoke-local/slice.json) lists what stays on
  (today: every Tinkers injection except the three the pinned Mixins refuses, and the PlutoLib features whose guards pass).
  `tools/slice_manifest.py` writes the reduced bundle; the full manifests are never edited to make a build pass.
  `tools/probe_injections.py` asks Mixins which injections the current Horizon accepts.
- PlutoLib's guards (`pluto-lib/tools/check_*_plan.py`) hash-pin official source on purpose. Never bypass one; a failing guard means that feature needs re-review.

## Checks and CI

- Line endings are LF everywhere (`.gitattributes`); `tools/check_line_endings.py` enforces it. In Python write files with `newline="\n"` (the lint rejects `write_text` without it).
- `tools/lint_forbidden_api.py`: no fork references, no non-official git dependencies; legacy direct host use is frozen by `tools/forbidden_api_baseline.json` (may shrink, never grow).
- Tests: keep only the necessary ones (see the roadmap, section 4). A test must fail when the behaviour it names is broken; never add one only to pass.
- CI (`.github/workflows/mixins.yml`) runs on pull requests: the checks above, the unit tests, then stages the slice and runs `cargo check`. It takes about 25 minutes, so work is verified locally first and CI is a second check. Documentation-only changes skip the build.

## Pitfalls seen here

- Windows Python writes CRLF with `write_text` unless `newline` is given.
- The launcher and Horizon are Windows GUI binaries: no console output unless redirected.
- Mixins re-prints edited Rust files from a token stream, which turns `0. ..=1.` into the invalid `0...=1.`; `tconstruct/tools/stage.py` rewrites that formatting in the staged copy only.
- In Bash, heredocs with backticks or apostrophes are mangled; write commit messages and PR bodies to files and use `git commit -F`, `gh pr create --body-file`.
