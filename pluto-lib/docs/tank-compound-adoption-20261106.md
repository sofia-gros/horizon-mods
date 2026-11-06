# Tank-owned compound patch adoption

Status: bounded native fixture passed at exact published source
`9ab6be063b832debd898f834343a07b9b1aa7064`. See
[tank compound validation](tank-compound-validation-20261106.json).
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

## Bounded verification

- Five source guards pin complete old/adopted method hashes, restore the original
  files, reject adversarial alias/opaque/raw-slot drift, and require real types.
- `tconstruct/tests/run_tank_compound.py` compiles both whole actual modules as
  exact source prefixes. It appends only test-only implementations with the
  frozen methods, renaming only their declarations. Native types/codec/helper
  modules and the one actual packaged inventory API are reused, not modeled.
- Five new required native tests passed: 308 Smeltery matrix comparisons,
  216 Casting matrix comparisons, exact unknown UTF-16/float row preservation,
  ten usable/stale root/nested wire refusal comparisons, and native map key /
  overwrite semantics. Source and proposal nonmutation are checked.
- The runner requires clean official Horizon at the compatibility pin, derives
  its Cargo lock from that host, accepts only known fixture/API package additions,
  and verifies source/API/prefix/lock identities before and after execution.
- The actual result was 146 passing Rust tests: five new boundary tests and
  141 existing imported module tests repeated. No tests failed or were ignored.
  The five source guards also passed on the exact required source export.
  No full host stage, game, save/reload, renderer or LAN test was performed.

The recorded original caller file hashes are provenance only. They are not
proof of live save/reload, multiplayer delivery or notification behavior.


## Exact execution and preservation

The fixture uses only the 50 required files from central v15 plus this nine-path
follow-up. All 50 file hashes and both complete production-module prefixes were
verified before and after. The one actual API crate was referenced by its real
central checkout path through an explicit fixture-input symlink, with all 28
API files and Cargo.toml matching v15. This does not substitute a copied registry.
Official Horizon was clean at `624c4f6265c02296fbe8a80f029dd0d987fb94c0`,
and the host-derived Cargo lock remained unchanged. The test ran locked/offline,
with one job, debug info disabled and incremental compilation disabled.

The run finished successfully in 5.67 seconds. The continuous native disk guard
never stopped it; minimum free space was 10,871,758,848 bytes, just above the
10 GiB + 128 MiB working threshold. This margin is not enough for further builds.

A capacity assumption was incorrect: identical package source/path/lock identity
did **not** imply artifact reuse. Cargo recompiled the actual API because the
resolved serde_json dependency fingerprint differed from its earlier cached
variant. The former artifacts were retained. Future capacity checks must account
for resolved feature/dependency artifacts or reserve recompilation space; metadata
identity alone must not be treated as proof that a build will be fresh.

Only the completed, idle dedicated test executable was subsequently removed,
after a separately authorized preservation-only recovery using a 10 GiB + 64 MiB
write floor. The complete binary, dep-info and proof records were compressed,
fsynced, read back and checked for hash, regular-file type, mode, uid/gid and
nanosecond mtime. The archive is
`.work/official-cleanup-evidence-20261105/tank-compound-completed-executable.tar.gz`,
1,931,866 bytes, SHA-256
`a23ca9a7fe1a0f8d1581a31db0ac3a514c5df0163d46f4a71d851c66fbc6de72`.
The binary working copy alone was removed after final no-process/handle/mapping
and hash checks; all source, logs, dep-info, API and other library caches remain.
The final measured free space after the receipt was 10,877,112,320 bytes.
The normal native-build threshold remains 10 GiB + 128 MiB.
