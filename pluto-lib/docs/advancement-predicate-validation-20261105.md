# Focused predicate facade validation, 2026-11-05

This slice provides the host-independent callback boundary for shared advancement
item matching. The API, lossless component DTO, registry and native adapter are
PlutoLib code. The actual tool/tank criteria and startup registration remain on
the separate TConstruct acquisition recovery branch, not in the shared package.

## Checks at this source cutoff

- Real shared Cargo crate: 17 tests pass, including no-callback/unowned fallback,
  handled-false, duplicate rejection, lexical conflict resolution, startup opt-in,
  callbacks outside the registry lock, UTF-16/numeric data preservation, metadata
  scopes and inventory component keys.
- Actual unified package: 4 tests pass. They check exact source bytes, real
  serde_json dependency/module inclusion, one canonical predicate adapter,
  default-off gates and no consumer IDs in shared code.
- Separate consumer branch: 23 production callback-policy tests pass using the
  real shared crate and its DTO. No synthetic host ItemView field is used.
- Official focused apply passed in 7.225 seconds: 1 crate, 3 modules,
  3 injections, no state or bridge declarations. The actual unified package was
  extracted, with only the advancement manifest subset selected; both real
  consumer modules and their startup registration were included.
- The resulting official stage passed `cargo check --offline --locked -p
  horizon-client --lib --no-default-features` in 83 seconds. This includes
  gameplay/startup and the client library, not just a fake predicate fixture.
  It covers the new default-off gate and callback boundary.
- The earlier check in advancement-validation-20261105.md remains a separate
  pre-gate proof. Neither check verifies the original acquisition PR heads.
- No game, LAN, full TConstruct bundle or 263-PR validation is claimed.

The verified focused official target is Horizon
624c4f6265c02296fbe8a80f029dd0d987fb94c0 with Mixins
e8cd0aa38cffef49cb2e088ef716baa107148c02. Only authorized official exports and the
mod repository are used; no host source edits or fork dependencies are required.

## Dependency impact

The existing pluto-inventory-api crate now declares serde_json = "1". Native
consumer fixtures must reference that actual Cargo package, not compile lib.rs
alone with rustc or duplicate it using #[path]. The generic transaction runner
now Cargo-tests a byte-for-byte copy of the complete real crate in a temporary
directory, isolating lock/build writes without replacing or excluding any API.
The existing single bundled-crate instance remains attached to horizon-client-lib;
gameplay accesses its existing reexport. No bundled crate depends on another
bundled crate or on the host.

## Exact proof cutoff and reproducibility

Shared source commit: `d1dbf7601bb84bb2479559ed4a99c1b0964cb8c9`.
Consumer wiring commit: `b259ae7d3e090df1f8f96ac796d6076b2925d24f`.
The latter is a feature-branch consumer change, not part of the shared foundation.
The independent brief review found no serious registry, fallback or lossless
conversion issue; it was a source review, not additional runtime evidence.

Build controls were `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`,
`CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=2`, and the official Linux sysroot include
path. The existing official host cache was reused. A disk guard preserved the
10 GiB floor; observed minimum free space was 11.948 GiB. The check completed
normally with exit 0 and the heavy build slot was released.

All four applied API crate files (manifest, lib.rs, advancements.rs and
advancements/predicates.rs) were byte-compared against the source cutoff and
matched. The package and key source hashes below bind the proof without implying
that other unified-library hooks or the whole consumer manifest were checked.

Archive SHA-256: `3b497d2796ed65f5530bb0a59a1f70b2d532a899b29ca35a881be35ff6933907`.

- `pluto-lib/mixins/pluto_native_creative/inventory-api/src/advancements/predicates.rs`: `2216bf87629864a4a965264d7b9e5cfa2db922b589a883962182b927a510d29e`
- `pluto-lib/mixins/pluto_native_creative/advancement-item-predicate.rs`: `af0512cfe7d8750dbff74328952f86e56fe646b25de1e76eeb68e9e5497ad3e0`
- `tconstruct/mixins/tconstruct/source-advancement-tool-predicate.rs`: `754d62cd0c28b82c298cab60f141c3d6356ee90cfd52aa8d2d02f58198f7d0eb`
- `tconstruct/mixins/tconstruct/source-advancement-tank-predicate.rs`: `0dc3525b17f34f9e471bd45308add4a77c24935f2ed3c66245a293695f79c177`
- `tconstruct/mixins/tconstruct/source-advancement-predicate-startup.rs`: `60b5e9b21e090d80fc066828f73f8b68413fc39a378600c6f29fcf43c01942ff`
