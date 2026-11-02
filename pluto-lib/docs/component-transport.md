# Item component transport

Horizon already carries custom data, lore and item models in its native stack conversion. PlutoLib does not add a second codec for these fields on the newer host. The older [item behavior demo](item-behaviors.md) still targets its original Horizon version and retains its original adapters; it has not been ported to the newer host.

## What has been checked

The [fixture](../fixtures/stack_transport_3a.rs) uses the actual Horizon `3a264943c7a3a3ed02e6ee608443f558d98f208f` stack encoder, wire codec and decoder. Two sequential conversions preserved a nested custom-data compound, lore, item model, item ID, count, damage and unbreakable, with each tested component appearing once. A decoded stack retained its cached wire patch after a count-only change. The two conversions call the shared codec twice; they do not execute the host and guest wrappers. Patch retention was checked before sending, not on an actual transmission. A real LAN connection and save/reload remain untested.

A separately supplied Tinkers fixture at commit `45497078ea4d3359c6b383bcca70f11674cfb373` preserved custom data, lore, item model, item ID and count in this conversion. Both tested pickaxes lost `minecraft:max_damage` and `minecraft:tool` when a newly constructed stack was encoded; the vanilla stone control retained its count. The two pickaxes therefore did not retain all components. The fixture checks the outgoing component IDs directly, using the host registry IDs for both fields. An experimental [max-damage adapter](../mixins/pluto_transport_3a/horizon-mixin.toml) adds the official integer component to a newly encoded stack without changing an existing component or wire patch; its helper also preserves a removal marker. The fixture verifies this narrow behavior. It is not included in the distributed bundle or verified in a game. The experimental adapter also encodes and decodes the standard 26.3 `tool` raw component. Its fixture covers the supplied pickaxes’ actual mining rules, tag-based and explicit block lists, and retention of unrelated patch entries across a count-only change. The host alone retains tool bytes but does not reconstruct the item component. The adapter is a prototype: unsupported tool fields, unresolved registry IDs, mutated server patches, and actual host/guest wrapper behavior still require investigation; if encoding fails, it currently falls back to the host, which can omit the tool component. The older adapters do not independently repair this gap. Do not remove or combine adapters on the strength of these checks alone; their cancellable hooks can also interfere with one another.

For accepted values, the fixture checks prepared generated and modified stacks through both slot codecs, including Tool, max damage, custom data, lore and item model. It does not run the guest or host wrappers. The pre-existing host limitation for unrecognised entries of a modified cached patch is distinct from this accepted-field contract. A complete replacement still needs real bidirectional LAN and save/reload checks. This result is not a claim that Tinkers or item behavior works on the newer host.

## Prepare a Tool component

When staging `pluto_transport_3a` with the native mod, call its entry before registering or changing a Tool component:

```rust
let canonical = horizon_client::pluto_tool_codec::prepare_tool(
    r##"{rules:[{blocks:"#minecraft:mineable/pickaxe",speed:2.0f}]}"##,
    |block_id| lookup_block_id(block_id),
)?;
// Store `canonical` as the item's `minecraft:tool` SNBT.
```

The callback resolves an explicit block ID against the current host registry and returns `None` when absent; tags need no numeric lookup. `lookup_block_id` stands for the consuming mod’s registry adapter. The result and errors are host-independent strings and `ToolError`; invalid SNBT, an unresolved block ID and unsupported fields are errors before registration. Apply the same entry on each Tool change. Store the returned canonical value, so the sender and receiver use the same stack and search identity. This entry is experimental and exists only in the staged native client; it is not in the released package.

## Prototype limits

The Tool wire fields are `rules`, `default_mining_speed`, `damage_per_block`, and `can_destroy_blocks_in_creative`. Each rule contains `blocks` and optional `speed` and `correct_for_drops`. Block sets may be a tag, one block ID, or a list of block IDs. Explicit IDs must resolve in the connection’s block registry. Integer and floating NBT numbers are accepted for speeds; integer NBT numbers are accepted for damage. Missing optional values use the host’s defaults.

Tool identity is based on the host’s saved NBT, while the wire format does not distinguish one block ID from a one-element block list or omitted defaults from explicit defaults. The experimental `prepare_tool` entry validates and converts both forms to the wire-equivalent saved form before a stack is created or registered. Fixtures verify that these canonical forms have the same stacking and Creative search key. Block tags remain distinct from explicit block lists. The automatic transport hook cannot mutate the source stack; it warns when that stack is noncanonical, so an unprepared stack may change identity on receipt.

Unknown fields, malformed values, oversized identifiers, and unresolved block IDs are not encoded. The adapter reports an error to stderr and falls back to the host, which may omit `minecraft:tool`; that is **not** a successful transfer. The injected encoder returns `Option<Stack>`, where `None` means an empty slot, rather than a recoverable error, so it cannot safely reject the send at this boundary. A production API needs a fallible, user-visible send boundary or an upstream host change. A changed server-origin stack invalidates the host’s cached patch. The prototype warns when this happens, but the host re-encoder can lose unrecognised additions or removals; it cannot yet guarantee safe retransmission. The original patch is retained while only the count changes. Do not use this experimental adapter as a production transport guarantee.

## Run the fixture

Use a disposable source checkout at Horizon `3a264943c7a3a3ed02e6ee608443f558d98f208f`; these commands add an example to that checkout. From `pluto-lib/`, set `HOST` to the checkout’s absolute path:

```sh
HOST=/path/to/disposable-horizon
mkdir -p "$HOST/crates/horizon-client-lib/pluto-fixture/fixtures" \
         "$HOST/crates/horizon-client-lib/pluto-fixture/mixins/pluto_transport_3a"
cp fixtures/stack_transport_3a.rs "$HOST/crates/horizon-client-lib/pluto-fixture/fixtures/"
cp mixins/pluto_transport_3a/{max.rs,tool_codec.rs,tool_encode.rs,tool_decode.rs} \
   "$HOST/crates/horizon-client-lib/pluto-fixture/mixins/pluto_transport_3a/"
cat >> "$HOST/crates/horizon-client-lib/Cargo.toml" <<'TOML'

[[example]]
name = "pluto_stack_transport_fixture"
path = "pluto-fixture/fixtures/stack_transport_3a.rs"
TOML
(cd "$HOST" && cargo run --locked --offline --profile fast --no-default-features \
    -p horizon-client-lib --example pluto_stack_transport_fixture)
```

Use a fresh checkout so the example is added only once. Omit `--offline` if the checkout’s dependencies have not been cached. Append `-- /path/to/cases.json` to inspect consumer stacks: the file contains a `cases` array with `id` and SNBT `snbt` fields. The fixture checks the helpers independently of Mixins; a plan alone does not establish that the transformed client compiles or runs.
