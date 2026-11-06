# WIP: latest official Pack8 CI controller

Source only, not triggered or executed. This is a separate controller for the
frozen latest fixture at `15397f42`, targeting official Horizon
`f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`. The old `55e4d1d0` Pack6 controller
was prepared/reviewed but never executed. The earlier `ce495a88` overlay8
succeeded only on its old624c scope; neither result proves these latest cases.

## Bounded execution contract

- One exact public push branch, standard ubuntu-24.04 runner, fixed checkout
  action, contents-read permission. No secrets, cache, uploads, self-hosted
  runner, added permissions or billing configuration.
- A pre-setup allowlist requires the exact repository, source base, branch,
  official URL/revision and new lock SHA256. Thirty controller checks run
  before toolchain setup; ten actual selected-source checks run after the
  approved clean official checkout and before locked dependency fetch.
- Official lock `0bc4234f3001d44df9d46fbb06df331d3f53b1af6b613ec9082baaec3d93c62f`
  seeds the fixture. Exact three root local patches are reproduced. Full lock
  identities and every metadata local manifest path are checked, including
  transitive native crates; registry substitutes or other roots are rejected.
- Cold real core/mods/client-lib/renderer and fixture `--test` compiler lines
  are required. All eight exact tests run once, zero ignored/measured/filtered.
  Public `read_lang_file` and actual native pack merging remain the only
  language owners; no parser/dictionary substitute is compiled.
- Continuous 10 GiB +128 MiB floor on all written filesystems and a 2 GiB setup
  reserve. Each child is in its own bounded process group, with TERM/KILL/reap
  cleanup and group absence checks; timeouts are failures, not successful skips.
- Original source and actual official checkout are sealed before/after,
  including ignored/untracked files and non-followed link targets. Fixture
  bytes, full file-set, resolved lock, metadata and generated OUT_DIR hashes
  remain fixed. Seals do not claim permission-mode or empty-directory identity.
- Test command `-vv` logs are rejected if recompilation occurred. This is
  post-execution detection, not a promise that rustc cannot start. Raw logs are
  validated in full before bounded console output; their hashes do not imply
  the ephemeral raw files can later be retrieved.
- A partial false receipt is printed on failure, with completed gates. Success
  requires the full mandatory gate set, source verification, and no error.

`derivation.json` records every literal edit from the old Pack6 run/preparer/
workflow; guard/receipt/process implementation is not silently rewritten. New
source-variable binding is `SOURCE` because the selected-byte checker does not
pretend its export is a Git checkout. The controller/preparer separately enforce
actual clean complete official Git identity before native setup.

## Scope

The six inherited cases characterize order, filters, definitely invalid bytes,
Mod whole-file replacement and selected locale. Two added cases characterize
latest native comment fallback and strict JSON scalar types versus lenient
string conversion. This is baseline verification, not translation-priority
repair. No Mod stamp-root/pruning operation or Game reload is called.

No Mixins apply, installed Game authority, Options locale installation, live
UI, retained Guide/LAN refresh, production assets, rendering gameplay, native
world-fluid contact effect or runtime TOML claim follows from this controller.
An independent source review and central serial-slot release are required
before creating its trigger branch. Existing source/checkpoints stay unchanged.

## Source preparation evidence

The 30 Python controller/derivation checks passed locally on 2026-11-06,
followed by the separate 10 exact selected-source checks. These are not Rust
execution or a latest dependency graph result. The frozen fixture received an
independent source-only review against its actual official parser/pack bytes;
no major API/type/oracle blocker was found. The new controller still requires
independent review and a central execution release.
