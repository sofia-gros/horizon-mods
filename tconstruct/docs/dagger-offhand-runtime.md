# Dagger offhand attack

This is the bounded ordinary-mob runtime slice of issue #379. It builds on
Padded #734 and does not complete all Dagger or dual-wield interactions.

## Implemented

- An intact, single stock Dagger in the offhand can consume the existing
  authoritative entity-use hook and attack that packet's named living mob.
  A normal item-use miss also consumes its independent offhand cooldown.
  Main-hand use keeps its existing precedence. Native callers swing once.
- Cooldown belongs to the player UUID, not the item or selected slot. Mantle
  uses `round(20 / attackSpeed)`, rejects a second attack in the same player
  tick, and allows partial-charge attacks on later ticks. Damage uses the
  exact elapsed fraction without the main-hand half-tick interpolation.
  Death and world clear discard this transient state; changing the Dagger
  does not refill it. No cooldown state is saved.
- Damage, attack speed, and knockback are evaluated on a cloned attribute map.
  Main-hand item modifiers are removed, the Dagger's source stat is added to
  the base, and retained status/armor/command multipliers are evaluated in
  their original order. The live equipment and inventory are never swapped.
- The existing primary mob attack runs once with the offhand still acting.
  Its damage, hurt/kill events, statistics, exhaustion, normal sounds,
  particles, native target resistance and network updates remain in use.
  The direct main-hand cooldown reset is restored afterward. Scoped depth
  prevents nested ordinary attacks from inheriting offhand damage or charge.
- Padded's source knockback reduction also applies inside this validated
  offhand scope. A landed fast-weapon hit shortens the mob's invulnerability
  timer using the source tool's attack-speed stat.
- Dagger's native registration uses a flint template, so the scoped path
  explicitly supplies one source melee-primary wear and the missing item-used
  statistic. It uses the existing Overslime/durability calculation and retains
  the broken offhand tool. Creative use does not wear it; misses do not wear it.

## Gates and limits

The live Dagger definition and Padded/Silky Shears definitions must match the
pinned source. Offhand Attack is the static source modifier, so a data-pack
JSON replacement is not interpreted. Current MELEE, MELEE_WEAPON and DURABILITY
item tags and resolved material traits are required. Broken stacks, malformed
levels/stats, count-two stacks, wire patches, enchantments and custom use or
attribute components are rejected.

Supported additional definitions are the exact source Cultivated, Dense,
Stringy, Preserved, Tanned, Reinforced, Lightweight, Ductile, Sharpness and
Swiftstrike definitions. Silky is excluded so Silky Shears cannot compete for
the entity interaction. Other damage, conditional attribute, looting,
interaction or static modifiers need their own ordered offhand dispatch.

For this slice the main hand must be empty or a plain, unenchanted vanilla item
other than a mace. TConstruct main-hand tools and custom-data main-hand items
are excluded because existing dynamic melee hooks still read the selected
main hand. This does not yet support two TConstruct Daggers or an arbitrary
TConstruct weapon alongside the offhand Dagger.

Targets use the existing ordinary vanilla-mob allowlist. PvP, armor stands,
vehicles, multipart bosses, dead/nonattackable entities, armor-based attacks,
Toy squeaking, and other entity families are deferred. Native interaction
range/border checks still apply. There is no offhand cooldown HUD or custom
break animation. Remote clients use the existing interaction/use packets and
server swing synchronization, without new prediction or custom networking.
Arbitrary third-party synchronous nested attacks are not claimed as fully
source-equivalent; their legitimate main-hand reset is not overwritten.

## Source and verification

- Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`:
  `OffhandAttackModifier`, `ToolAttackUtil`, `ToolAttackContext`, generated
  Dagger and modifier definitions in the existing reference archives.
- Mantle `17c287ad52f7b10154e07fa7570dc7bccd74ca2b`:
  `src/main/java/slimeknights/mantle/util/OffhandCooldownTracker.java`.

Brief static review checked hook signatures, native acting-hand routing,
attribute APIs, Padded ordering, retained wear and main-hand cooldown isolation.
New rule tests cover repeat gating, partial recharge, cooldown rounding,
fast-weapon timer reduction and conservative definition gates; they were not
run. No build, planner, tests, CI inspection, version checks or merge performed.
