# Native canonical item decode validation, 2026-11-05

Source: `78cdf6dc814a278bad1dd74e6cf8ffe322d6943e`, based on the grouped
`999468ef` foundation plus the separately validated nested page guards.
Official Horizon: `624c4f6265c02296fbe8a80f029dd0d987fb94c0`.

## Scope and result

The actual native adapter and both complete production Smeltery callers passed
124 fixture executions: 84 in `run_item_canonical_decode.py`, then 40 in
`run_inventory_projection.py`. These include existing production tests and
repeated shared tests, not 124 newly added cases. Eleven source guards passed.

Both runners used the actual local `pluto-inventory-api` Cargo crate and clean
pinned official item/world dependencies. Their lockfiles were seeded from the
official host lock; resolved package identities were checked against it. The
canonical fixture additionally uses the real official `horizon-client-core`
for the existing fuel code. No native type or codec was replaced with a mock.

Execution was serialized in the shared target with two jobs, debug info and
incremental compilation disabled. The continuous 10 GiB + 128 MiB guard did not
stop the run. Minimum free space was 11,320,881,152 bytes; terminal status was 0.
Local evidence: `/tmp/pluto-canonical-proof.log`.

## Exact boundary

`decode_item_canonical` returns the native decoded stack only when re-encoding
compares equal to the supplied native `Tag`. It does not replace the codec or
compare serialized bytes. Tag types, UTF-16 and float/double bit patterns use
native Tag equality.

Exactly three previous decode/re-encode checks delegate to it:

- The proposed single item in `prepare_single_insert`.
- Each nonempty existing raw slot scanned by that same function.
- Each proposed replacement in `Smeltery::inventory_to_nbt_preserving`.

The caller retains its empty compounds, count/component checks, raw-slot
comparison, aliases, progress restrictions, page rules and commit semantics.
Cannon and Casting Tank's expected-decoded-item comparisons and unchanged
opaque-slot exceptions are deliberately untouched.

## Regression evidence

Four required native tests check:

- Adapter equivalence to the previous decode/re-encode expression across zero
  and high counts, all four nested-stack fields, malformed values, missing
  counts, aliases, unknown raw fields, UTF-16 and floating-point bit patterns.
- Canonical saved Tag equality remains different from live snapshot identity
  and save eligibility. Serialization may already have omitted retained wire
  or runtime-only bundle selection before this decoder receives the Tag.
- Complete single-insert outcomes equal the frozen pre-extraction function,
  including returned raw storage, placed item and remainder snapshots.
- Complete inventory-patch outcomes equal the frozen pre-extraction method,
  including empty/rejected inputs, progress restrictions and preserved fields.

The baseline functions in the test fixture preserve the bounded bodies from
`1cbbe64f`; they are test comparisons, not alternate production codecs.

This is isolated native type/caller evidence. It is not a complete TConstruct
package apply, live mutation, native hook dispatch, world or LAN proof. The
new native helper has not been included in the earlier `999468ef` package
proof. Later potion-catalog consumer changes require their own combined-source
fixture run.
