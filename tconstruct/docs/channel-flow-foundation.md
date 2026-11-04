# Channel flow calculation foundation

This is an unconnected bounded amount planner, not a registered block or a
playable Channel. The pinned TConstruct f4e77d43 source uses a 40 mB buffer,
locks newly filled fluid until the end of its next tick, tries the downward
output first at up to 10 mB, and otherwise divides the unlocked amount among
all configured horizontal outputs with a 1–10 mB per-side limit. The caller
must supply horizontal sides in source iteration order and each independent,
compatible sink's simulated capacity. The planner makes no world mutations.

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
