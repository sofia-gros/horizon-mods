# Tank-owned compound patch adoption

Status: source-only candidate. Native fixture **not run** at this checkpoint.
This is a storage-group follow-up, not an update to the frozen v15 library proof
or validation of an entire TConstruct PR. No PR state is changed.

## Existing contract, two actual callers

`SmelteryTank::to_nbt_preserving` now delegates its retained row `amount` and
outer `capacity`/`fluids` writes to existing `pluto_nbt_storage::patch_compound`.
`CastingTank::to_nbt_preserving` delegates its outer `tank`/`slots`/`redstone`
writes. No new public API, registry, manifest, native hook or ownership is added.
Pluto remains optional for other mods; these TConstruct consumers use Pluto.

Each caller still owns all field names, aliases, parsing, validation, raw-slot
identity and permission to produce a proposal. The exact original complete
method is frozen before replacement. A source guard restores both original
files from those methods and checks their immutable hashes. Production changes
are limited to the two methods, eleven added and eleven removed lines.

## Preserved semantics and limits

- Native `Compound` is a `BTreeMap<JavaString, Tag>`. Existing keys are unique,
  sorted by exact UTF-16 units, and `insert` overwrites a previous value. This
  adoption makes no statement about duplicate occurrences in an unparsed NBT
  byte stream. The helper uses the same map and ordered update semantics.
- Smeltery aliases, strict UTF-16 fluid ID conversion, exact fluid/tag row
  matching, opaque row deletion refusal, canonical proposal validation and
  new-row behavior remain in their original order.
- Casting aliases, tank roundtrip eligibility, original raw slots, unchanged
  slot preservation and refusal to overwrite a noncanonical old item remain
  unchanged. Root and nested retained wire still fail the existing save guard.
- Unknown keys, unpaired surrogate keys/strings, nested values and float bits
  are copied as native Tags, without a lossy string conversion or codec roundtrip.
- The original is not mutated, including failed proposals. No world storage,
  dirty marking, persistence, networking, held-item update or notification call
  changes. This is not a transaction/rollback/actor or physical-slot authority.
- The Smeltery outer map clone occurs later, when the patch is produced, rather
  than before the remaining pure validation. Returned values and validation
  order are retained; allocation/OOM timing is not claimed to be identical or
  recoverable. No user callbacks occur inside either method.

## Prepared verification

- Five source guards pin complete old/adopted method hashes, restore the original
  files, reject adversarial alias/opaque/raw-slot drift, and require real types.
- `tconstruct/tests/run_tank_compound.py` compiles both whole actual modules as
  exact source prefixes. It appends only test-only implementations with the
  frozen methods, renaming only their declarations. Native types/codec/helper
  modules and the one actual packaged inventory API are reused, not modeled.
- Five new required native tests are prepared: 308 Smeltery matrix comparisons,
  216 Casting matrix comparisons, exact unknown UTF-16/float row preservation,
  ten usable/stale root/nested wire refusal comparisons, and native map key /
  overwrite semantics. Source and proposal nonmutation are checked.
- The runner requires clean official Horizon at the compatibility pin, derives
  its Cargo lock from that host, accepts only known fixture/API package additions,
  and verifies source/API/prefix/lock identities before and after execution.
- Existing imported module tests may repeat. They will be reported separately
  from the five new tests. A later granted bounded run needs no full host stage.

The recorded original caller file hashes are provenance only. They are not
proof of live save/reload, multiplayer delivery or notification behavior.
