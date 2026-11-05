# Blaze melting and the first Blazing Blood

This slice restores the Blaze acquisition path from the pinned TConstruct
`EntityMeltingModule`, `EntityMeltingRecipe`, and `HeatingStructureBlockEntity`:

- A formed Smeltery or Foundry with a recognized fuel tank checks its interior
  on tick 12 of its twenty-tick entity cycle.
- The source Blaze recipe matches `forge:blazes`, deals 2 damage, and supplies
  20 mB of Blazing Blood when the native damage call accepts the hit. Ordinary
  lava fuel can bootstrap it: the source lava recipe uses 50 mB for 100 fuel
  duration at 1000 degrees and rate 10. No existing Blazing Blood is required.
- A tank fuel search is simulated first. Newly selected fuel is consumed only
  when at least one hit is accepted. Existing active fuel remains authoritative.
- Dead, removed, outside, blacklisted, or Fire-Resistant Blazes are not melted.
  Native invulnerability, armor, effects, cooldown, absorption, death protection,
  incoming-damage hooks, knockback, RNG and mob events remain in the host path.
- Fire-immune mobs use the actual `tconstruct:smeltery_magic` identity; the other
  source is `tconstruct:smeltery_heat`. Magic is Witch-resistant, not an armor-
  bypassing alias of vanilla Magic. Heat is fire damage. World tag overrides
  remain effective, within the host bridge's supported environmental semantics.
- The source intentionally allows partial fills and discards overflow, including
  a full tank. An accepted hit still consumes newly selected fuel in that case.

## Resources and transactions

`content/reference/tconstruct-entity-melting.zip` retains 103 exact generated
JSON resources from TConstruct commit `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`:
entity tags, damage definitions and damage tags. Per-file hashes are in
`upstream-entity-melting.sha256.json`; the existing TConstruct MIT license applies.
The original recipe remains in the existing recipe archive. Both manual and
packaged preparation retain the additional source layer.

The current world's source recipe snapshot already gates config and conditions.
Native, source and enabled world-pack tag/definition layers are then resolved in
priority order. Replace/removal semantics and actual registered entity/fluid
identities are respected. Snapshots are cleared on world teardown and parked-
world reload. Unsupported damage tags, unknown output, ambiguous matching
recipes, malformed predicates or unavailable context disable the action.

The maximum possible controller/fuel write is checked before damage hooks.
Native hits run once on staged Mob clones, retaining RNG and event queues.
Controller/tank/fuel metadata commits before live mobs are replaced. A failed
commit grants no fluid and changes no live mob health, RNG or events. Native
incoming-damage callbacks themselves remain normal callbacks, not rollbackable
external effects. They receive no mutable Game/world handle. Unknown fluid-row
extensions and malformed UTF-16 identifiers disable new entity damage, based
on the original pre-refresh data rather than its decoded projection.

## Dependency and bounds

This requires the separately reviewed native environmental-damage host bridge,
currently `sofia-gros/horizon` commit `3a04811822551b0c9717119a1ba8de833c999fc1`,
based on official `0c7b5b61a5e746d621afffba919b12aeccf9c0fa`. The compatibility
pin and CI source repository are explicit; this is not compatible with an
unmodified official host. Keep the dependent PR draft until combined integration
passes and the required host branch is available.

Only native Blazes are enabled here. Other living mobs, players, generic Liquid
Soul fallback, potion/NBT outputs, and ambiguous source recipe ordering remain
unimplemented. This does not claim complete entity melting. Resource, parser,
actual native mob-hit and save-state roundtrip tests are automated. A live
survival capture/carry/melt/drain sequence, full world save/reopen, and two-player
LAN presentation have not been exercised.

Source: https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity
