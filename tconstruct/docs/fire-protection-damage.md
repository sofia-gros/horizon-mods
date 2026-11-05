# Fire Protection damage reduction

Related to #379. Stacked on the protection-acquisition and Fire/Blast secondary
runtime chain. This connects Fire Protection to the native player protection
stage, after armor and Resistance, rather than scaling incoming damage early.

## Implemented

- Intact supported equipment contributes 3 protection points per effective
  Fire Protection level, retaining fractional five-unit progress. One level
  gives a 12% reduction at the protection stage before reaching the default cap.
- Support 13 correct-slot stock worn armor classes plus Travelers Shield,
  Plate Shield and Battlesign in either hand. Shields need not be raised.
  Current worn/held tags, material/fixed traits, upgrades, durability and the
  complete stock modifier definition are read on refresh. Unsupported records
  fail closed. Hands precede feet-through-head in the source equipment order.
- The source enchantment module normally adds vanilla Fire Protection, then
  cancels its protection points before adding the TiC coefficient. This bridge
  adds only the final TiC contribution and leaves saved native enchantments
  alone, avoiding an extra two points per integer level.
- Broken worn armor is a distinct source case: virtual Fire Protection still
  exists, but EquipmentContext skips both modifier hooks. Such pieces contribute
  only 2 points per completed integer level on the current vanilla fire tag.
  Broken held shields do not contribute. Partial levels below one add nothing
  on broken armor. The general 3-point modifier applies only while intact.
- Capture damage-type tags from native catalog defaults, the source packs and
  enabled packs in priority order. Respect replace/remove and nested tags.
  The source fire tag includes fire, lightning and source shock. Project concrete
  IDs onto the native catalog, so unimplemented source-only damage types cannot
  accidentally match another type. Required missing nested tags, cycles and
  unreadable resources disable the projection rather than guessing membership.
- Keep the extra table in transient StatusFlags state, separate from the native
  enchantment table. It is rebuilt after equipment/status/tick updates and source
  reload, then copied naturally into the native damage environment. Repeated
  refresh does not accumulate points; removal replaces the table with no bonus.
- The native damage kernel retains armor/toughness, wear, Resistance, enchantment
  bypass, absorption and the default 20-point / 80% protection cap. No protection
  enchantment is written to the item and no early incoming-damage hook is used.

## Boundaries

The shared stage now also covers [Blast Protection](blast-protection-damage.md),
[Magic Protection](magic-protection-damage.md), and
[Projectile Protection](projectile-protection-damage.md).
Other protection modifiers, Overshield,
Boundless and custom protection-cap changes remain unsupported. Exact mixed-modifier float ordering,
negative protection and extended caps remain unsupported. Damage predicates that
bypass protection follow the native kernel/catalog; this does not replace those
native rules with separately reloaded source bypass tags. Other held tools,
custom armor classes, mobs and remote-client prediction are not covered.

Native armor wear occurs before this protection stage. The source Forge event
uses inverse armor math and compensating wear to achieve the same intended
stage, so exact event/stat/wear rounding parity is not claimed. Actual damage,
reloads, guest equipment swaps, status resets, saving and LAN remain untested.

Sources: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`fire_protection.json`, `ProtectionModule`, `EnchantmentModule.Protection`,
`ModifiableArmorItem`, `EquipmentContext`, `ToolEvents` and `ArmorUtil`.

Four regression cases were added for source points/broken equipment and damage
tag projection, replacement, removal and failure cases. They were not run.
Brief static review only; no tests, builds, planner runs, CI inspection, version
checks or merges. Publication remains paused.
