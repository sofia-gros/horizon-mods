# Crafting-match admission validation, 2026-11-05

Official targets: Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and
Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`. No fork is used.
[Machine-readable evidence](crafting-match-veto-validation-20261105.json)
records source hashes, exact fixture identities, process outcomes and scope.

## Passed focused evidence

- 7 fixture tests: one new grouped native matching/consumer-path test, four
  unchanged TConstruct planner tests, and two existing component-view tests.
- 65 real `pluto-inventory-api` unit tests, including five new matching API tests.
- 6 Python source/package/composition tests. They verify exact packaged sources,
  interest-before-component-capture, and fail-closed missing, competing,
  spoofed, drifted and byte-mismatched plan boundaries.
- Actual packaged read-only Mixins plan: 2 crates, 41 modules, 28 injections,
  2 states. The crafting-match guard passed that plan, including the native
  fingerprint `4df48b30b17f2c4d735e45b54ae2e5c7d23f679c993b3d942c07f5aaecbf401c`.
- Fixture source/fragment and dependency-identity checks passed again after tests.
  Native/API runs used the real Cargo package, the official locked identities,
  debug0/incremental0/jobs2 and a continuous 10 GiB + 128 MiB free-space floor.

The native fixture executes exact official shaped/shapeless matching,
`Input` cropping, ingredient testing, fixed template creation, find and known-ID
match bodies. It uses real native items/components and unchanged shared native
adapters. It covers no-provider/uninterested continuation, deterministic veto,
valid and mixed/missing material, wrong shape/item/count/result metadata, holes
and offsets, reverse conversion, unavailable retained-wire components,
continuation to a later candidate, known-ID matching, unchanged inputs and
native output construction on Pass. Pure API tests cover unlocked reentrant
registration and all interested callbacks running after a denial.

The preserved consumer reference is MOD commit
`8c56bb52b6b77fae6115e894af9e369a5c3f9e08`, not a production port. The original
gate and planner are byte-checked against that commit. Its material policy is
kept only in TConstruct test artifacts.

## Explicit limitations

The fixture models the HEAD call wrapper, the small outer Recipes container,
external native ingredient providers and special-recipe endpoints. Its special
test proves delegation versus denial only, not native special-recipe behavior.
Unused files inherited from the placement fixture are not additional executed
placement tests. No live UI, LAN, consumption or performance test ran. Shared
component facts are not full private StackData/Wire identity.

The focused planner alone does not establish native type correctness. That gate
is now closed by the separately recorded complete package below. The preceding
placement batch `c55f4985` did not contain this new seam.
No TConstruct production consumer, recipe ownership ledger or automatic
component-sensitive allocation was added.

## Complete packaged native gate

Combined Pluto cutoff `529ad7229422121d6bda28e5b5c7af5279fa0c76` passed actual
plan, all nine mandatory guard groups, apply, and official host library/binary
checks. Archive SHA-256:
`3c769c1399095cea630b2de17dc14a6a520876aa27c6bd0735d2e07bbc9b7fed`.
The package has 2 crates, 57 modules, 32 injections and 3 states. Host check
completed in 68 seconds with no disk stop; minimum free space was
11,297,124,352 bytes. The combined suite reports 159 Python tests including the
98 real inventory API tests; these overlap earlier counts and must not be added
as new distinct tests.

The archive hash and all four frozen matching API/adapter sources were checked
independently against this feature's source. The complete plan's matching guard
was also re-run successfully. Evidence is under the coordinator's
`grouped-v10b` artifact directory, identified in the JSON report. Bounded source
review by two other workers found no serious issue; it adds no runtime coverage.

This closes the library native type/composition gate only. It does not activate
TConstruct policies, validate the original stacked PRs, establish live crafting
or LAN behavior, or validate the next station-ingredient parser slice.
