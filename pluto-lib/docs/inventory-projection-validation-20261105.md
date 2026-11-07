# Read-only inventory projection validation

The inventory/storage group passed its focused projection proof at MOD head
`9c2cab95981845aa27fa3af77d46494e7b622c96`, based on the previously checked
combined foundation `0dd927ef26fa7b6ef3452873fe6085beb533bd15`.

The native fixture used clean official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`, seeded resolution from its lockfile
and rejected unknown dependency identities. It depended on the one actual
`pluto-inventory-api` crate and compiled both production game/library re-export
paths plus the real Chest, Smeltery page and layout modules. No API module or
registry was stubbed or disabled.

## Boundary

`projection::project_slots` owns ordered cloning through explicit optional
source indices. Empty, unmapped and unavailable cells yield empty view cells;
repeated indices yield repeated clones. There is no machine schema, host type,
mutation or implicit layout in this API.

TConstruct retains the complete mappings: its 54-cell transport, chest widths
and visible rows, side-chest padding, Smeltery scrolling and reserved container
slots. Its click/drag, acceptance, raw-slot and authoritative commit rules are
unchanged. Projection clones stack data directly, including opaque wire and
runtime selection; it does not serialize or require save eligibility.

## Results

- Complete actual inventory API Cargo package: **37 tests passed**, including
  four new generic projection tests.
- Real native projection fixture: **38 tests passed**. Required cases check
  both chest mappings against their previous loops, retained opaque data,
  source cells remaining unchanged, Smeltery layout/reserved cells and existing
  ghost/stale-page rejection. Earlier production module tests also ran.
- Projection source guards: **3 passed**.
- Native packaging checks: **8 passed**. The new source module is packaged
  through the existing actual crate, with no new dependency or native module.
- `git diff --check` passed.

The two Cargo runs total **75 passing test executions**, not 75 new behaviors.
The native fixture repeats six compound-patch tests from the shared storage
fixture and existing Chest/Smeltery production tests. Native mapping regression
matrices include empty, partial and final pages, beyond-end offsets and opaque
stack data without changing layout policy.

Commands:

```sh
python3 -m unittest discover -s pluto-lib/tests -p test_inventory_transactions.py -v
python3 tconstruct/tests/run_inventory_projection.py
```

The proof used two jobs, no incremental compilation or debug information, and
the continuous 250 ms disk guard. It exited zero with minimum free space
11,025,821,696 bytes, above the 10 GiB + 128 MiB threshold.

## Limits

This is exact-source, focused evidence. It does not apply the complete new
library bundle, compile the entire TConstruct composition or establish live
world/LAN behavior. Existing page commit and item-save eligibility gates were
not changed by this read-only extraction; a projected view is not authority to
write a real or virtual slot.

## Separate nested no-opaque guard adoption

A subsequent consumer-only follow-up at
`5e353d1f67b5aee195b06e6f0533b80759e98ea0`, based on combined foundation
`999468ef38bd542e2e4dc8f83205f26462ad6e33`, passed **40 native fixture tests**
against the same clean official host and lockfile. Six projection and
pickup-routing source guards also passed.

Smeltery's existing `no_retained_wire` and `prepare_single_insert` checks now
delegate to Pluto's recursive `contains_retained_wire` through the real
game/library re-export paths. This closes their root-only gap without changing
the existing codec, raw-record canonicality checks, layout, count splitting,
access rules, snapshot equality or commit order.

Two new required regression matrices cover all four native nested-stack
fields, both usable and stale retained wire, and opaque data in either the
sent or expected page. Ordinary representable nested stacks still insert with
the same one-item/remainder counts; rejected proposals leave their inputs
unchanged. Read-only projection continues to retain opaque data.

The run used the same continuous guard and two-job/debug-free settings. It
exited zero with minimum free space 11,371,921,408 bytes. The 40 executions
include the earlier projection fixture cases; they are not 40 new tests. The
complete-package proof at `999468ef` predates this consumer delegation and must
not be presented as validation of the whole updated TConstruct composition or
live host/guest behavior.
