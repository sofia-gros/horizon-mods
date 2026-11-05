# Duct filter draft integration

This draft requires the pre-removal host boundary in
[sofia-gros/horizon#22](https://github.com/sofia-gros/horizon/pull/22), pinned to
`dbe80ca7de510dad736b5a9c58813499cdeb5df6`. TConstruct and PlutoLib pins move
together. **Do not merge this draft independently into main**: the host changes
must first be combined with the separate world-fluid and other host candidates,
then the combined revision must be validated and pinned consistently.

Seared and Scorched Ducts accept one filled supported bucket, Copper Can or
lantern. Use the item on an empty Duct to insert one, and use an empty hand to
retrieve it. The world's `tconstruct:duct_containers` tag accepts the item or
its crafting remainder, following the pinned source `DuctItemHandler` rule.
Empty containers and unsupported representations are not consumed. This direct
interaction is a limited replacement for the source's single-item menu.

Faucets select the first controller layer matching both fluid ID and exact NBT,
regardless of layer order or container volume. No filter means no new fluid is
drained. Existing reserved Faucet packets retain their identity when the filter
changes. Registered fluid checks and existing network potion restrictions stay
in force. The filter item itself is never drained.

An occupied Duct cannot be broken, replaced, pushed or overwritten through the
host's guarded removal/command paths. Retrieve its filter first. Unknown native
metadata, legacy Forge inventory, undecoded raw entities, unexpected live
inventories and named Ducts also block these operations instead of discarding
or duplicating state. No implicit legacy migration is attempted. The guard is
before loot and inventory mutation, not a late `set_raw` rejection.

The filter is stored once in custom block-entity metadata. A zero-slot chest is
only the host save projection; metadata uses the existing changed-state guest
sync. Direct raw-storage writes outside the host guard remain outside this
safety boundary.

## Verification boundary

Implementation and a brief static review only in this phase. New focused tests
cover exact NBT selection, empty filters, item roundtrips and opaque envelopes;
they are queued for the final validation phase. No current comprehensive test,
CI result, or merge is claimed. The source-style menu, filter model rendering,
automation/capability handlers, in-game save/reload and actual LAN behavior still
need completion or verification. This does not close the entire Duct item in
#379. The older #695 draft is the no-filter safety fallback and must not be
merged over this implementation without reconciliation.
