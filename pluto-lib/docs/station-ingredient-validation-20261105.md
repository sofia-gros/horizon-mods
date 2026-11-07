# Station ingredient boundary validation, 2026-11-05

Official targets are Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and
Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`. No fork is used.
[Machine-readable evidence](station-ingredient-validation-20261105.json)
records immutable source/fixture identities and exact process outcomes.

## Passed focused gates

- One grouped native fixture test exercises exact official Ingredient
  parsing/of/test and the station ingredient body with actual native Items
  catalogs. It compares the test-only callback with the exact old consumer
  helper across item/tag/union/difference, invalid grammar, limits and explicit
  empty cases. It also verifies native fallback, unchanged nested native
  structure, two distinct current registries, missing-versus-empty tag facts,
  conflict rejection and unknown returned-item rejection.
- 73 real inventory API unit tests passed, including eight new parser tests.
  They cover order, empty versus Unsupported, duplicate IDs, deterministic
  multiple-claim conflicts, unlocked callbacks/stable snapshots, callbacks
  after conflict, and result item/byte/uniqueness/existence bounds.
- Six Python source/package/composition tests passed. All three API/adapter/head
  source files are included byte-exact in the generated archive.
- An actual packaged read-only official plan passed with 2 crates, 42 modules,
  29 injections and 2 states. The mandatory parser guard passed that plan and
  captured native function fingerprint
  `209d222eef7391e83c44d179f2e640c5a1dd7735fc40407adfa5faa3cb376b5f`.
- Fixture source/fragment/locked dependency identities passed again after tests.
  The real Cargo package was used, with no stub or conditional exclusion of API
  modules. Jobs2/debug0/incremental0 and the continuous 10 GiB +128 MiB disk
  floor were maintained; no disk stop occurred.

The exact source reference is MOD commit
`8c56bb52b6b77fae6115e894af9e369a5c3f9e08`'s
`source-stonecutting-ingredient.rs`. Its grammar, limits and TConstruct marker
remain solely in the consumer test artifact. Bounded independent source review
found no serious issue; that does not add runtime coverage.

## Limits

The fixture models the HEAD call wrapper and unrelated native custom ingredient
provider endpoints. It does not execute the full StationRecipes loader,
component-sensitive external providers, station UI, crafting, LAN or saves.
Unused files inherited from the placement fixture are not extra executed
placement tests. This service returns flat item-ID ingredients only; it is not
a nested/custom component matcher or output/remainder authority.

Plan success alone does not prove generated native type correctness. The
combined package below now closes that gate; the preceding matching batch
`529ad722` did not include this parser. No production TConstruct registration
or old hook removal is included, and no original stacked PR is declared validated.

## Complete packaged native gate

Combined Pluto cutoff `3ba060dfca5e567bf9ff0b4853f429a221564c25` passed actual
plan, all 11 mandatory guard groups, apply, and official host library/binary
checks. Archive SHA-256:
`e42240e75471359eb9b3c82486e30a1940ba5308c1688fccda25bc83acd22890`.
The package contains 2 crates, 61 modules, 33 injections and 3 states; 5,042
staged files were sealed. Host check completed in 64 seconds, with no disk stop
and minimum free space of 11,120,369,664 bytes. The combined suite reports 187
Python tests including 106 real inventory API tests. These overlap the earlier
focused results and must not be counted as additional distinct tests.

The complete archive hash and all three frozen station API/adapter/head sources
were checked independently. The station guard was re-run successfully against
the actual combined plan. The preceding v11 attempt stopped before apply on an
unrelated terrain module inner-doc header; its corrected v11b cutoff is the
successful one. No station runtime source changed for that correction.

This closes library packaging, composition and native type validation only.
It does not activate TConstruct or establish full station loading, gameplay,
LAN/save behavior, component-aware matching or dynamic result/remainder writes.
