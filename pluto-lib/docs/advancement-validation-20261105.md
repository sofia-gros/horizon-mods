# Focused advancement bridge verification (2026-11-05)

## Default-off follow-up

The default-off atomic opt-in and earliest adapter `Continue` gate were added
**after** the official apply/type-check reported below. That host proof is for
the pre-gate implementation, not a claimed check of this new follow-up. Follow-up focused checks passed: **11 shared API tests** and **3 package/guard tests**,
with incremental compilation disabled. The guards check the disabled path
returns before snapshots, encoding and event state writes. No new official host
apply/type-check or runtime check was performed for this follow-up. Step 3 consumer
startup activation remains pending; the shared library must not opt in from a
predicate or silently on behalf of a consumer.

## Pre-gate target and scope

- Official Horizon: `624c4f6265c02296fbe8a80f029dd0d987fb94c0`.
- Official Mixins: `e8cd0aa38cffef49cb2e088ef716baa107148c02`.
- Source-only exports obtained from the official repositories. No upstream files
  were modified, and no fork resources were accessed.
- Actual unified PlutoLib archive was built and extracted. A focused manifest
  selected its existing inventory API crate, inventory export and new inventory
  event adapter. Two TConstruct item-predicate hooks were added as consumers.
  This subset has one crate, one module and three injections.

## Pre-gate results

- Official Mixins apply: **passed**, including packaged API source and native
  target resolution. The original host `ItemView` layout stays unchanged.
- Transformed official `horizon-client` gameplay library check: **passed** with
  `cargo check --offline --locked -p horizon-client --lib --no-default-features`,
  two jobs and dev debug information disabled. Final cached check took 55.56 seconds.
- Shared inventory API: **10 tests passed**, including five advancement-context
  regressions for exact key/type/thread identity, nested scopes, panic cleanup,
  user-defined Clone/Drop reentry, zero-sized keys, and component-sensitive keys.
- TConstruct predicates: **23 tests passed** using the actual shared facade.
  Fixture metadata is separate from the official-shaped `ItemView`, not an
  invented `ItemView.custom_data` field.
- Unified archive contract: **2 tests passed**, including deterministic output,
  exactly one shared API crate and byte-exact inclusion of the new API source.

The final latest check followed a cancelled first attempt: the initial offline
attempt lacked one cached dependency, then its online retry was stopped for the
inventory audit (exit 130). Neither was counted as a pass. The resumed final
check above completed successfully.

An earlier prototype passed a focused check on old official Horizon 0c7b5b61 with
Mixins ddd9700b (without the final facade). That historical result is not evidence
for the final facade or any original open PR head.

## Behavioral boundary and remaining work

The generic facade has no host or TConstruct imports. TConstruct owns all tool,
material, modifier and tank criteria. PlutoLib's latest adapter retains native
slot bounds, candidate selection, authority gates, player conditions and the
normal progress/reward path while passing complete encoded component snapshots
and detecting metadata-only inventory changes.

This is **not** a full TConstruct bundle migration, verification of all 19 stacked
acquisition PRs, merge-readiness result, or a gameplay/LAN test. Other inherited
hooks, stonecutting target identities, source/recipe integration, GUI behavior,
per-player LAN transitions and performance with large nested inventories remain
outside this focused proof. Metadata scopes currently cover inventory predicates
only; other item-predicate callers must fail closed without a scope. No CI status
or merge action is asserted here.

## Pre-gate checked source fingerprints

- `pluto-lib/mixins/pluto_native_creative/inventory-api/src/advancements.rs`: SHA-256 `0744e68282933fdce33ab4e626e43ec19e054efc93880e3ca9e55fc8465774cc`.
- `pluto-lib/mixins/pluto_native_creative/inventory-advancements.rs`: SHA-256 `c8d276c5129a524fad5737030acc2adbe46c07a52790c9c80e010894296645d4`.
- `tconstruct/mixins/tconstruct/source-advancement-tool-predicate.rs`: SHA-256 `c3da19fe9f6ea7403300be9aab5c52d8c15887fc479d53706c50880744abcef3`.
- `tconstruct/mixins/tconstruct/source-advancement-tank-predicate.rs`: SHA-256 `da3b16954a21f5ebfc1712e16305937faac75c7d8f56a557e7e911d5f5c4277f`.

Packaged PlutoLib source archive: SHA-256 `06f685d8ce26ef5d4778330ed10ccb2979d93939ceff60fea763d3b8967dc481`.

## Default-off follow-up source fingerprints

- `pluto-lib/mixins/pluto_native_creative/inventory-api/src/advancements.rs`: SHA-256 `16952dc9bd508496c99a9d1b218ea81e512539c8ffc7b7cc793b861262ae34a8`.
- `pluto-lib/mixins/pluto_native_creative/inventory-advancements.rs`: SHA-256 `5728349cc5513bf22f7628d9887a84b73482775eccfe30766f6ea388cf1459bc`.


## Later grouped callback proof

The [predicate facade proof](advancement-predicate-validation-20261105.md) records
an additional latest-official apply/type-check that includes the default-off gate,
shared predicate callback adapter and real consumer startup wiring. It supersedes
the earlier gate-unchecked limitation for that exact focused source cutoff only;
it does not change the original-head audit classifications or certify the whole
acquisition stack.
