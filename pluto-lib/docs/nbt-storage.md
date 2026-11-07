# Patch native storage without discarding extensions

`horizon_client::pluto_nbt_storage` stages changes to Horizon NBT compounds.
It owns no machine schema and performs no persistence. A consumer supplies the
fields it owns and validates its own aliases, values and authoritative state.
This native adapter is part of the shared inventory/storage feature group.

## Explicit field changes

```rust,ignore
use horizon_client::pluto_nbt_storage::patch_compound;
use horizon_world::nbt::Tag;

let saved = patch_compound(&original, [
    ("progress", Some(Tag::Int(7))),
    ("pending", None),
])?;
```

`Some(value)` sets a field. `None` removes it, including an already absent field.
Updates apply in iterator order; if a key repeats, the last update wins. The
original is borrowed and never mutated. Non-compound originals return `None`
before the iterator is consumed.

Keys may be `&str` or owned native `JavaString` values. Untouched keys and values
are cloned without string decoding, numeric conversion, schema normalization or
re-encoding. In particular, unpaired UTF-16 surrogates in extension names or
values and floating-point payload bits are retained. Explicit raw-key updates
also use `JavaString` directly, never lossy UTF-8 conversion.

## Copy only owned fields

```rust,ignore
use horizon_client::pluto_nbt_storage::patch_owned_fields;
let saved = patch_owned_fields(&original, &proposal,
    &["progress", "pending"])?;
```

Both arguments must be compounds. Only the listed fields are copied. A listed
field missing from the proposal is removed from the original; unlisted proposal
fields are ignored. Unlisted original fields remain unchanged. Repeated owned
keys apply in their supplied order, with the same final value from the proposal.
No aliases or field names are implied by the library.

Nested records are patched explicitly by the consumer, then included in an
outer patch. This is not a recursive merge and does not infer list, inventory,
item identity, empty-slot or migration semantics.

## Consumer boundary

TConstruct uses these operations for its Tank, Faucet, Melter, Casting and
Smeltery record patches. It retains:

- Exact owned-field lists and ordering, alias handling and identifier validation.
- The distinction between omitting an optional field, requiring one, retaining
  an unknown field and overlaying all fields supplied by a raw proposal.
- Its existing Faucet serializer's explicit empty-compound fallback. The shared
  API itself does not normalize malformed input into an empty compound.
- Existing item save checks and raw-slot preservation, including unchanged
  opaque items. Snapshot equality and save eligibility remain separate.
- World/chunk checks, callback ordering, machine and recipe policy, persistence,
  commit ordering and rollback.

The result is only a proposal. Validate data before calling, recheck live state
after callbacks and commit through the authoritative storage path. The helper
does not lock storage, make a multi-record write atomic or authorize a write.

## Validation scope

Direct tests use official Horizon's actual NBT types. They cover ordered
set/delete updates, missing and repeated keys, non-compound rejection, ignored
unowned proposals, untouched input and opaque UTF-16/nested/float data. Existing
machine preservation fixtures exercise the consuming paths. Record the exact
MOD and official host revisions with a completed test run; source extraction
alone is not packaged apply, full-client or live-world evidence.

### Grouped official-host result, 2026-11-05

The compound batch passed on combined MOD head
`947cdfb7ec4396991010630acef30895f216c205`, using clean official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`. All five fixture manifests used the
actual shared Cargo inventory API and verified dependency identities against
that host's lockfile. No predicate/event modules were copied or disabled.

- Native transaction and NBT patch fixture: **10 passed**, including all six
  direct compound tests.
- Cannon/Casting Tank fixture: **164 passed**.
- Casting preservation and real store fixture: **66 passed**.
- Seared reinforcement and related production modules: **232 passed**.
- Faucet state/runtime fixtures: **88 passed**, including required extension,
  opaque-slot and lossy-identifier preservation cases.

Total: **560 passing test executions**, with shared module tests repeated across
runners. These are not 560 unique scenarios. The grouped source checks passed
41 Python preservation/wiring tests, native package integrity checks and the
injected-module header check.

The run used two jobs, disabled incremental compilation and debug information,
and monitored free disk space every 250 ms. It exited successfully without
triggering the 10 GiB + 128 MiB stop threshold. This proof does not apply the
complete mixin bundle, compile the full client or exercise live-world/LAN play.
It does not cover the subsequent partitioned proposal-staging extraction.

Smeltery's tank-only and inventory-only proposals use `patch_compound` for their
final owned `tank` or `slots`/`progress` writes. Their existing decoded-state
comparisons, aliases, canonical item checks and progress restrictions remain
local. This does not introduce additional serialization of unrelated fields:
the tank proposal retains raw inventory, fuel and extensions unchanged. It is
still a staged Tag proposal, not an authoritative storage commit.
