# Grouped storage and potion fixture validation, 2026-11-05

Immutable MOD cutoff: `d8216e569ed29f9874a64688588b2b19d92ee8af`.
Official Horizon: `624c4f6265c02296fbe8a80f029dd0d987fb94c0`.
The grouped source includes canonical item decoding, nested Smeltery save
protection, the read-only potion catalog and their actual fixture wiring.

## Result

Four bounded native fixture runners passed 474 test executions:

- Canonical item/state: 85.
- Chest/Smeltery projection: 40.
- Cannon/held/casting: 164.
- Drain/container/fuel: 185.

This includes existing production tests and repeated shared tests; it is not
474 new independent cases. All required-case checks passed. Fourteen Python
source guards passed separately after the documentation-only snapshot wording
correction, with no runtime changes.

All four runners required the clean official host, used the actual
`pluto-inventory-api` Cargo crate and checked resolved dependency identities
against the official lockfile. Native item/NBT/core types, codecs and the Pluto
potion catalog adapter were real. Existing machine/world/storage callback
models remain explicitly modeled in the Cannon and Drain fixtures; this is
not live-world or native Game integration evidence.

## Semantics covered

The canonical helper still preserves exactly the previous native Tag
re-encoding comparison. Zero/high-count, nested-stack codec distinctions,
unknown raw fields, malformed values, aliases, UTF-16 and float-bit cases
remain covered by the old/new expression and complete-caller parity matrices.

The later serializer adoption intentionally strengthens one boundary:
`Smeltery::inventory_to_nbt_preserving` now calls shared `encode_item_lossless`
and then retains its independent canonical Tag check. It rejects retained wire
inside each of the four native nested-stack fields, whether currently usable
or stale. The regression confirms that the former outer-stack-only method
accepted these cases and the current method rejects them without modifying
inputs. This is loss prevention, not an old/new equivalence claim for opaque
nested data. Ordinary representable items, empty slots, alias refusal, progress
rules and existing raw-slot preservation remain unchanged.

Dedicated potion fixture wiring imports the actual adapter only in consumers
that need the native catalog. Generic storage/projection fixture dependencies
were not expanded merely to provide the alias.

## Execution and scope

The serial run used two jobs, debug info disabled and no incremental
compilation. A continuous 10 GiB + 128 MiB disk guard stayed active. Minimum
free space was 11,165,335,552 bytes, terminal status was 0, and the guard did not
stop the run. Evidence: `/tmp/pluto-storage-potion-proof.log`.

Three older Faucet runtime runners still generate their lockfiles without
checking every dependency identity; they were deliberately not run or counted
in this pinned proof. Their mechanical source wiring is present, but their
runtime verification remains separate. Overlord/ranged runtime fixtures were
also not part of this run.

The cutoff was clean and unchanged before and after execution. This does not
apply the complete TConstruct bundle, execute a live transaction or prove LAN
behavior. The snapshot helper's private Wire-baseline limitation is documented
separately; persistence rejects all retained backing regardless.

## Strictly pinned Faucet follow-up

At later combined cutoff `d039d95c634851bc2ee30c2aa11180e58b43a314`, the three
previously excluded Faucet runners were hardened and executed. They now reuse
`lock_fixture_to_host` and `run_locked_fixture`: the actual API crate and clean
official source are checked, Cargo resolves against the official lock, and each
named route case must explicitly pass. Cargo failure, missing/ignored cases and
an empty successful test run cannot satisfy the boundary gate. Six driver/unit
guards passed separately.

All 459 executions passed:

- Tank/casting: 149.
- Tagged/local: 148.
- Smeltery/tank: 162.

All 22 required route cases ran and passed. Counts include existing/repeated
shared tests and must not be interpreted as 459 new independent cases. The
actual native item/NBT/core types and shared potion adapter were used; modeled
world/store/structure/recipe behavior remains limited as each runner describes.

The clean cutoff was unchanged before and after execution. The serial shared
cache run used two jobs, no debug info and no incremental compilation. Terminal
status was 0; the continuous 10 GiB + 128 MiB guard did not stop it. Minimum and
terminal free space were 10,999,992,320 bytes. Evidence is
`/tmp/pluto-faucet-locked-proof.log`.

This closes the three runner provenance/runtime gaps explicitly excluded from
the earlier474-case group. It does not rerun those other cases or add a live
transaction/LAN claim. The complete Pluto package's separate v6b apply/check at
this cutoff also passed, but full TConstruct composition remains outside these
bounded fixtures.

## Final owned compound write delegation

At source `23cdf0a1f34a51510a4d9d91b7e845bf2e1a167c`, Smeltery's tank-only and
inventory-only proposal methods replaced only their final Compound clone and
owned-field insertions with existing `pluto_nbt_storage::patch_compound`.
No new API, schema interpretation or commit path was introduced. All prior
state/raw-slot/alias/progress checks and the nested save guard remain.

The exact-source canonical fixture passed 86 tests, including a new comparison
with the previous whole tank method and inventory outcomes. It covers unknown
raw item fields retained during tank-only writes, unchanged refusal during
inventory rewriting, malformed roots, aliases, unrelated state changes,
progress restrictions, unpaired UTF-16 keys and floating-point bit patterns.
The original inputs remain unchanged. Six source guards passed separately.

This follow-up ran with the same official host/lock checks, actual crate/types,
serial cache and continuous 10 GiB + 128 MiB floor guard. Exit status was 0; all 86
passed with none failed/ignored/filtered. The guard did not stop it; minimum
free space was 10,966,097,920 bytes. Evidence:
`/tmp/pluto-smeltery-compound-proof.log`.

This is distinct from the frozen d039d95c package/Faucet proof, and does not
retroactively change its source cutoff. It remains bounded proposal-method
evidence, not a live commit or full TConstruct composition result.
