# Item component transport

Horizon already carries custom data, lore and item models in its native stack conversion. PlutoLib does not add a second codec for these fields on the newer host. The older [item behavior demo](item-behaviors.md) still targets its original Horizon version and retains its original adapters; it has not been ported to the newer host.

## What has been checked

The [fixture](../fixtures/stack_transport_3a.rs) uses the actual Horizon `3a264943c7a3a3ed02e6ee608443f558d98f208f` stack encoder, wire codec and decoder. Two sequential conversions preserved a nested custom-data compound, lore, item model, item ID, count, damage and unbreakable, with each tested component appearing once. A decoded stack retained its cached wire patch after a count-only change. These checks do not execute the private client send method, a real LAN connection or save/reload.

A separately supplied Tinkers fixture at commit `2d40af6955b9695a04c479ce54abdd462f4e7448` preserved custom data, lore, item model, item ID and count in this conversion. Both tested pickaxes lost `minecraft:max_damage` and `minecraft:tool` when a newly constructed stack was encoded; the vanilla stone control retained its count. The two pickaxes therefore did not retain all components. The older adapters do not independently repair this gap. Do not remove or combine adapters on the strength of these checks alone; their cancellable hooks can also interfere with one another.

A complete replacement needs the host to preserve all relevant components, registry mappings and safe server-origin wire patches, followed by real bidirectional LAN and save/reload checks. This result is not a claim that Tinkers or item behavior works on the newer host.

## Run the fixture

Add `fixtures/stack_transport_3a.rs` as an example of a disposable checkout of the pinned Horizon `horizon-client-lib`, with `horizon-protocol`, `horizon-world` and `serde_json` available from its existing dependencies. Run that example with default features disabled. Pass an optional JSON file containing a `cases` array with `id` and SNBT `snbt` fields to inspect consumer stacks; no consumer IDs or assets are built into the fixture. The fixture records the observed `max_damage` and `tool` gap so a host change prompts a fresh assessment.
