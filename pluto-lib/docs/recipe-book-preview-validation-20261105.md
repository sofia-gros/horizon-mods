# Recipe preview validation, 2026-11-05

## Scope and source

The broad recipe group adds an explicit ghost-preview capability and read-only
component-preserving ingredient overlay helper. It does not add persistent
RecipeIndex ownership or automatic consumer activation. Source and contracts:
[recipe-book-preview.md](recipe-book-preview.md).

Official Horizon: `624c4f6265c02296fbe8a80f029dd0d987fb94c0`.
Official Mixins: `e8cd0aa38cffef49cb2e088ef716baa107148c02`.
The clean official checkout's origin and immutable revision were verified.
No fork source or upstream edits were used.

The bounded fixture uses byte-exact native Ingredient/Shape, Entry/Display,
Book declarations, entry layout methods and Book::items_of, real official
ItemStack/world dependencies and unchanged production helpers. The outer Game,
active-menu lookup and ghost-call recording are modeled explicitly. This is not
an actual UI, inventory transaction, native shell or game/LAN test.

`prepare_recipe_preview_fixture.py` records whole official source hashes,
unique slice delimiters and fragment hashes, all copied API/helper hashes and
exact local Cargo package paths. Resolved dependency identities must match the
official lock except the fixture and the single real API package. Verification
was repeated after the run. No stubs replace ItemStack or the production observable snapshot comparison.
That comparison does not expose Wire's private decoded baseline; equal stale
patch bytes may conceal different private baselines. The fixture is not proof of
full private identity or permission to commit a stored selection.

## Results

- 6 focused fixture tests passed: current-entry validation; stale field,
  observable component, retained-wire and runtime-bundle rejection; local/book/geometry
  gates; changed-menu rejection; native overlay layout and alternatives.
- 55 real API unit tests passed, including 2 new generic overlay tests.
- 4 deterministic source/package guards passed, including actual archive bytes,
  host-independent API shape and no automatic hook/state registration.
- Both Cargo commands exited 0 in about 1 second each, offline and locked,
  debug0/incremental0/jobs2. The continuous 10 GiB + 128 MiB guard did not stop
  either run; minimum free space was 11,180,269,568 bytes. Existing unused-patch
  warnings remain and are not a failure or a warning-free-build claim.
- A brief independent source review found no serious issue in exact entry
  comparison, fallback/layout or the ghost-only boundary. It is not a lifecycle
  audit. A whitespace-sensitive source test assertion was corrected; production
  sources did not change between fixture preparation and these results.

The fixture is under `.work/official-advancement-recovery/recipe-preview-fixture`:
`fixture-sources.json`, `test.log`, `result.json`, `api-test.log` and
`api-result.json`. The adjacent JSON captures reproducible source hashes.
Prepare a fresh fixture against the named clean official checkout, then run
`cargo test --offline --locked --manifest-path OUTPUT/Cargo.toml --lib` and
repeat with `-p pluto-inventory-api`. Use the preparation tool's `--verify-lock`
mode afterward. Coordinate the native build slot and disk guard.

## Remaining gates

The actual complete package gate now passed as recorded below. There is still
no TConstruct consumer wiring, component-sensitive placement grant, original PR
certification, save/reload or multiplayer proof here.

The RecipeIndex state-constructor feasibility plan failed with
`E_UNSUPPORTED_CONTROL_FLOW` before apply; evidence is in
`.work/official-advancement-recovery/recipe-index-feasibility/plan.log`.
No persistent index ledger, loader copy or prefix/pointer workaround was
implemented. This is an explicit remaining lifecycle blocker, not a test
failure hidden by the narrower preview proof.

## Complete native package gate

Complete PlutoLib source `d039d95c634851bc2ee30c2aa11180e58b43a314` includes the
unchanged preview API and both native helpers. The actual archive bytes were
compared with the focused fixture's production sources and matched exactly.
On the same official 624c/e8 pair, plan, mandatory preflight, apply and
`horizon-client` library plus binary check all passed. The check finished in
74 seconds, exited 0, and did not trip the disk guard; minimum free space was
11,025,891,328 bytes. Warnings remain.

The complete package contains 2 crates, 43 modules, 27 hooks and 3 supported
states. These are whole-library counts, not additions by preview. The combined
source suite passed 94 Python guards and 68 API tests. Archive SHA256:
`8122fa6b4f72cdb07af92a95f7c9d350ace97cc9308fd2b643c7b70d8ef896b7`.
Evidence is in the central integration worktree's
`.work/plutolib-foundation-proof/grouped-v6b`: `cutoff.json`, `plan.json`,
`native-preflight.log`, `apply.log`, `host-check.log`, `result.json` and
`api-source-tests.log`. This closes the actual-module placement/type gate, not
a consumer migration, gameplay, persistence or placement-authority gate.
