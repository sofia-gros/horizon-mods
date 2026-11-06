# Placement veto validation, 2026-11-05

This is the next slice of the broad recipe API group, outside the verified
preview cutoff d039d95c. Contracts and path limitations are in
[recipe-placement-veto.md](recipe-placement-veto.md).

## Bounded behavior proof

The fixture pins clean official Horizon
624c4f6265c02296fbe8a80f029dd0d987fb94c0 and uses exact native Ingredient,
Recipe, Cooking, Placement, Entry and Book declarations, native Placement::of,
Stacked item picker, and the bodies of Recipes::placement, Menu::place_recipe,
Game::place_recipe and the Book placement projection. Renamed native entry
functions plus explicit HEAD wrappers model injection for this fixture. The
actual Mixins plan is a separate gate below. Outer Game/menu state, slot-write
endpoints and network sends are modeled, not a game or LAN process.

The full real inventory API is one Cargo path package. ItemStack and protocol
packet types are real official dependencies. Preparation and post-run checks
verify immutable origin/head, exact fragment selectors and hashes, copied source
hashes, local package paths and dependency identities against the official lock.
Source-order assertions separately cover local UI, both LAN menu paths, the
canonical pre-mutation menu query, ordinary/native-mod dispatch and Book's
craftable projection. Those assertions are not executed LAN tests.

The test-only TConstruct policy is outside PlutoLib. Its material classification
is compared with the exact old marked(Recipe) body and marker constant from MOD
8c56bb52b6b77fae6115e894af9e369a5c3f9e08. No production registration is installed.

Results:

- 2 focused tests passed. One grouped path test covers no-provider local and
  remote fallback; reference-policy agreement for ordinary, material, nested
  and shapeless ingredients; Book exclusion; menu denial before mutations for
  normal/shift and survival/creative requests; successful preview without
  allocation; five presentation-failure cases; remote denial; unknown recipes;
  cooking precedence/name normalization; and Silent dominance. The other tests
  native ingredient views and uninterpreted custom/nested metadata.
- The remote-display-without-source-marker case deliberately preserves packet
  sending. It proves the stated limitation: the server must register the
  authoritative definition policy. Client-only presentation is not protection.
- 60 real API tests passed, including 5 new registry/ordering/fallback tests.
- 6 source, actual-package and composition-guard tests passed.
- Both Cargo runs exited 0 in about 1.2 seconds each, offline and locked,
  debug0/incremental0/jobs2. The continuous 10 GiB + 128 MiB guard did not stop
  either; minimum free space was 11,039,608,832 bytes. Warnings remain.

Evidence: `.work/official-advancement-recovery/recipe-placement-fixture-v4/`
contains source identities, native/API logs and results. Reproduce with
`prepare_recipe_placement_fixture.py`, then run the fixture's locked/offline
Cargo tests and repeat for `-p pluto-inventory-api`; verify with `--verify-lock`
afterward. Its source-only consumer fixture is under
`tconstruct/tests/rust/recipe-placement-policy/`.

## Actual planner and composition proof

The actual archive containing the unchanged helpers was planned against the
pristine same official Horizon and Mixins
e8cd0aa38cffef49cb2e088ef716baa107148c02. Plan passed in 16.22 seconds, with
2 crates, 40 modules, 27 hooks and 2 supported states in this feature-branch
package. These are not the latest central aggregate counts. The complete-plan
placement guard then passed against exact native fingerprints and helper bytes.

`check_recipe_placement_plan.py` rejects missing, spoofed or competing hooks at
both boundaries, phase/binding/fingerprint changes and stale source/helper
bytes. The coordinator wired it into the central mandatory preflight for the
complete-package gate below. Earlier cancellable HEAD hooks and RETURN modifiers could
otherwise bypass or restore a denial; exclusive ownership is conservative.

Evidence: `.work/official-advancement-recovery/recipe-placement-plan/` contains
the archive, actual plan and log/result. The first invocation selected the
archive root rather than its mixins directory and failed with E_MANIFEST before
planning. The corrected directory passed without any production-source change.
A whitespace-sensitive Python source assertion was also corrected before its
final pass. These were harness issues, not hidden runtime failures.

## Remaining scope

Actual apply and official host library/binary checking passed in the central
grouped package below. No full TConstruct activation, runtime gameplay,
save/reload, server deployment, component-aware allocation or persistent
RecipeIndex ownership is claimed. Independent brief source review found no
serious lookup, fallback or denial/preview ordering issue; it was not a full
consumer or gameplay audit.

## Complete native package gate

The complete PlutoLib source
`c55f4985001999b7ea9aa34f7470e8725a5684af` passed plan, mandatory preflight
(including the placement guard), apply and official horizon-client library plus
binary check on the same 624c/e8 target pair. The five placement API/helper
sources inside the actual archive exactly match this slice's bounded proof.

Archive SHA256:
`47dcc0d20c79beb6a592db4385b729e1019e87de0c752cac4430a658ea4e4f61`.
The whole package contains 2 crates, 51 modules, 30 hooks and 3 supported states.
The grouped source suite passed 130 Python tests and 93 real API tests. Host
checking finished in 84 seconds, exit 0, minimum free space 10,979,622,912 bytes,
without a disk-guard stop. Warnings remain.

Evidence is under the central integration worktree's
`.work/plutolib-foundation-proof/grouped-v8/`: cutoff/plan/result JSON,
api-source-tests.log, native-preflight.log, apply.log and host-check.log. The
coordinator sealed 5,039 staged files for this cutoff. This closes the actual
module/type and complete-plan composition gate, not consumer startup activation
or a live player/guest placement test.
