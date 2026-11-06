# Owner-bound cloud insertion: source audit

Target: official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and
Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`. This audit precedes the
new insertion implementation. It does not enable a TConstruct caller.

## Source ownership and contact semantics

Pinned TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`:

- [FluidEffectContext](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/fluid/FluidEffectContext.java)
  carries an explicitly supplied nullable living source entity, separately from
  a projectile and player. `MobEffectCloudFluidEffect.makeCloud` assigns that
  exact entity as owner. It does not select the nearest/current player.
- [BurningLiquidBlock](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/block/BurningLiquidBlock.java)
  checks fire immunity and positive immersion in its own fluid type, then sets
  fire, applies lava damage, and plays a randomized sound only on accepted
  damage. A generic block overlap is not that fluid-height test.
- [MobEffectLiquidBlock](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/block/MobEffectLiquidBlock.java)
  requires positive fluid-type immersion and a living entity, creates its
  effect and empties curative items before applying it. Cadence, deduplication,
  burn values, effect identity and cure policy remain TConstruct decisions.

Official `block_contact::Event::Inside` is a static notification with dimension,
entity ID/type and a touched block. It lacks a mutable Game/actor and the source
fluid-type immersion height. The proven read-only custom-fluid View can measure
its own geometry; it does not establish Forge's exact immersion/callback policy.
No contact-effect hook is enabled in this group.

## Exact official authority observations

All paths below are in the pinned official `crates/horizon-client/src/`:

- `game/native_lifecycle.rs`: `epoch()` spans Opening through Closing, so an
  equal epoch alone does not permit insertion. The new read-only child adapter
  observes the existing private `OpenAttempt` and accepts only `Ready` with
  `IntegratedServer` or `DedicatedServer`. It owns no lifecycle state or hook.
- `game/integrated.rs` and `game/dedicated.rs`: `WorldOpenReady` follows the
  completed open; close enters Closing before final epoch retirement. Pending
  opening, failed/closed opens, Closing, local mirrors and remote clients reject.
- `game/mod_api_host.rs::native_authoritative` is checked in addition to
  `remote.is_none()`. A caller-provided DTO is not used to prove authority.
- `game/native_entity_refs.rs::native_level` and `native_level_height` resolve
  the actual current world/epoch. Insertion supports only the level in place,
  never implicitly swaps a parked dimension or flushes side effects.
- `native_entity_target` checks the current epoch, a placed guest in the actual
  LAN roster, or a current-level entity list; removed mobs reject.
  `native_entity_level` must resolve to the requested current level.
- `commands/players.rs::command_guest` requires the actual LAN hub. A guest
  frame whose hub is temporarily taken out by `as_guest` is deliberately
  unsupported: matching numeric `lan_acting` fields is not a fallback lookup.
  A resolved acting player additionally must match the native motion entity,
  guest connection/entity/UUID frame. Other placed guests use their roster
  connection. Mobs use their resolved native ID. Armor stands, dragons and
  nonliving targets are unsupported rather than projected into another owner.
- `mod_api.rs::EntityRef`: IDs are not reused within an open and moving an
  entity to another dimension assigns a new ID. Lookup proves a currently
  resolvable entity, not that it caused a particular user action. The consumer
  still must pass the source entity from its proved action context.

## Rejections and side-effect order

The service requires an explicit `Unowned` request or an explicit native entity
handle. Its resolved owner/guest must agree with descriptor attribution; no
owner is invented and no disagreement is repaired. The cloud center's chunk
must be loaded (X/Z only, matching a chunk-loading condition, not a new height
or replaceability rule). Source admission rules still belong to the caller.

Before entity allocation: check server/Ready/current level, actual owner lookup,
attribution agreement, loaded center and signed network-ID successor; construct
all native payload allocations and reserve vector capacity. Only then call the
native `take_entity_id`, assign it, push to `breath_clouds` and return the handle.
There are no callbacks or remaining fallible Result operations in that commit
sequence. There is no rollback promise after allocation/publication, and no
network delivery, persistence, ticking or gameplay proof from this ordering.

The existing TConstruct `source-fluid-cloud.rs` has no explicit owner argument;
`source-splashing-block` hooks `Game::use_block_transformer` without an actor
handle. It therefore remains unchanged and disconnected from the new service.
The existing duration-times-four/quarter-scale pair and overflow admission also
remain consumer policy. Guest action integration needs additional source proof.
