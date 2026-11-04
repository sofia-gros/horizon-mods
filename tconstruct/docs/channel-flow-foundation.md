# Channel flow calculation foundation

This is an unconnected bounded amount planner, not a registered block or a
playable Channel. The pinned TConstruct f4e77d43 source uses a 40 mB buffer,
locks newly filled fluid until the end of its next tick, tries the downward
output first at up to 10 mB, and otherwise divides the unlocked amount among
all configured horizontal outputs with a 1–10 mB per-side limit. The caller
must supply horizontal sides in source iteration order and each independent,
compatible sink's simulated capacity. The caller must check the same fluid
identity and metadata, reject aliased sinks, and atomically revalidate the full
plan before commit. The planner makes no world mutations.

Before connecting it, implement and validate block registration and source
shapes, placement and side controls, powered neighbour updates, actual fluid
identity and metadata, live per-side capabilities, alias-safe atomic recheck
and commit, lock persistence, saves, destruction, chunk boundaries, rendering
and LAN. The planner rejects invalid numeric buffers rather than interpreting
unknown saved data. It does not model flow animation or claim external-handler
transaction semantics. Existing blocks and recipes are unaffected.

Run the standalone model tests with:

```sh
rustc --edition=2021 --test tconstruct/tests/rust/channel_flow_plan.rs -o /tmp/channel-flow-tests
/tmp/channel-flow-tests
```

## Unconnected native state codec

The Channel state codec stores a native-format 40 mB tank and a non-negative
`locked` amount no greater than its fluid amount. Its patch operation retains
unrelated outer and nested tank fields, the fluid metadata compound and the
source `is_flowing` field as opaque NBT. It rejects malformed values and
legacy Forge `FluidName`/`Amount`/`Tag` spellings, including mixed layouts, and fluid IDs that cannot be represented
losslessly as UTF-8; it does not migrate or interpret Forge saves. Unknown extension fields are
preserved as data, but their semantics are not understood. No world read,
write, block entity registration, transfer, chunk unload or destruction is
connected. Save/load, old-world compatibility and LAN remain unverified.
The eventual caller must validate the fluid resource-location syntax and its
presence in the current world's registry. It must also ensure an extension has
not assigned different meaning to the owned `tank`, `fluid`, `fluid_tag`,
`amount`, or `locked` keys; this codec cannot distinguish a same-name collision.
