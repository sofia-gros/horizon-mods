# Material projectile speed and accuracy

This draft targets the original Longbow and Crossbow projectile speed and
accuracy using their saved material stats. The source bow multiplies charge by
velocity; crossbows multiply their projectile speed by velocity. Both use
`3 * (1 / accuracy - 1)` for inaccuracy. Existing Horizon random generation is
retained. A registered stat or a successful planner probe does not demonstrate
that the emitted arrow uses it.

Representative Linux gameplay using runtime `f898d875dad20b4a3d3752a75b910258197a2fab` confirmed actual arrow speed and spread for both weapons, source fractional low/partial/full charge, main/offhand, source three-arrow fan, ammunition, mob hits, survival wear and broken retention, repair and save/reload, hosted LAN guest shots, and vanilla bow/crossbow firing. A Tinker Station swap from wood to cobalt limb changed Crossbow `tic_velocity` from 1.0 to 1.05 and measured arrow Z velocity from approximately 2.838 to 2.984; the replacement persisted after reload. Several comparisons used synthetic stat data to isolate the shared formula, and are not a test of every material. The speed samples include normal flight drag.

Official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab` applied in 198.89 seconds; the client/ctl build succeeded in 1m24s. Four scoped Rust tests, 44 TConstruct Python tests and 7 root Python tests passed. Windows and material combinations beyond the representative cases are unverified. Firework rockets, other conditional stat modifiers, armor actions, PvP area attacks, crop shears loot and trim remain unfinished.

Official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab` rejected
`release_bow` MODIFY_ARG at `arrow.shoot_from_rotation` with
`E_UNSUPPORTED_CONTROL_FLOW` (closure context), and `shoot_crossbow`
MODIFY_ARG at `arrow.shoot` with the same code (reference temporary lifetime).
A separate `add_projectile` HEAD probe was accepted, but runs after random
spread has already been applied. It must not merely scale the noisy vector:
that would not reproduce source accuracy or multi-shot direction. A scoped shot context carries the shooter and source direction to that boundary. Firework rockets use a separate launch path.
A draft scoped shot context now reruns arrow launch with source speed, spread
and fan angles at that boundary using Horizon's RNG; it consumes additional
random draws after the original vanilla launch. The representative arrow scenarios above have been built and tested. Firework rockets remain unimplemented. The draft permits source-valid low-charge shots through the existing vanilla
dispatch threshold; the representative low-charge case passed; remaining conditional stat modifiers are incomplete. See the Maintained implementation and its verification limits below.

## Maintained velocity

Bronze's Maintained trait now contributes to Longbow and Crossbow arrow velocity
at firing time. For remaining durability above half the maximum, it adds
`((remaining - maximum / 2) / (maximum / 2)) * level * 0.08 * velocityMultiplier`.
The bonus is zero at or below half durability. Repair restores the bonus on the
next shot; it is not baked into the saved base velocity. Rebuilding after a
material swap replaces the stored trait level rather than adding another bonus.

This follows the pinned [Maintained definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/maintained.json)
and [ConditionalStatModule](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/modules/behavior/ConditionalStatModule.java).
Its flat adjustment has priority 100, before Dwarven's priority 85 altitude
adjustment. Consequently Dwarven's high-altitude penalty also scales the
Maintained bonus. The existing velocity multiplier includes the tool definition
and Netherite upgrade. Both existing firing hooks read the changed calculation;
no new hook registration is required.

Focused Rust tests cover both bows, full/three-quarter/half/broken durability,
repair, repeated rebuild, trait removal, definition/Netherite multipliers and
Dwarven's altitude penalty. The local 41-test run used the saved older host's
`ItemStack`/NBT libraries (the former client items module aliased as
`horizon_items`), not a newly built fd6 client. **Official current-target build,
actual arrow measurements, save/reload and LAN checks for this change remain
pending.** The existing broken-weapon firing guards are unchanged.

For the combined-client check, rebuild a Bronze-limb bow through the Station or
Creative material path so it acquires the new fields, then compare full, 25%,
and 50% damage at the same charge and altitude. At level 1 and multiplier 1, the
velocity bonuses are 0.08, 0.04 and 0.0 respectively. Repair and repeat, swap out
the Bronze part and confirm removal, save/reopen, then repeat a representative
LAN guest shot. Add a Dwarven part and check below sea level and at the full
altitude penalty. Record actual arrow velocity with the same observation delay;
ordinary flight drag changes later samples.

Bronze is a compatibility material: this change does not add a native tin or
Bronze acquisition path. Use a clearly labelled diagnostic fixture if the
material is unavailable in the test world; this is not a survival progression
claim. Keen remains outside this batch: its original Quartz assignment is to
ammo, and adding a bow-only number would not implement the material ammunition
path. Other conditional projectile modifiers and armor effects remain incomplete.

## Airborne projectile velocity

A bow with the Airborne trait adds `0.1 * velocityMultiplier` to arrow velocity
when its holder is off the ground, out of water, not riding and not climbing.
The source gives the flat bonus once regardless of trait level. It runs before
Dwarven's altitude adjustment. The single-player path uses the existing host
climbable query, including its ladder and trapdoor rules; if the query fails,
the bonus is not applied. The LAN guest projectile effect is temporarily
unavailable until the guest's water, climbing and riding state is verified.
The mining portion is tracked separately in [mining traits](mining-traits.md).

Focused Rust checks use the saved older host's item and NBT libraries; they do
not validate the current host hook or gameplay. For the combined client, compare
both bows at equal charge and altitude while jumping, on the ground, in water,
on a ladder and riding; repeat after changing the material. Confirm save/reload
and check the guest path separately. Use a labelled material fixture if natural
acquisition is unavailable. No new gameplay or LAN result is claimed.

## Solid projectile damage

Invar's Solid trait adds 0.75 projectile damage per level, scaled by the
tool definition's projectile-damage additive multiplier. It combines with
Diamond and Ductile during stat rebuilding and is removed on material swap;
it is not added again when the same tool is rebuilt. This implements only the
projectile-damage portion of the [original Solid definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/solid.json).

Current-target compilation and gameplay are pending. Invar is a compatibility
material; this does not add its natural acquisition path. Verification should
cover both bows, zero/one/multiple Solid levels, definition multipliers,
Diamond and Ductile combinations, repeated rebuilding, material replacement,
and save/reload. The trait's protection, movement and gravity portions remain
separate work.

## Raging projectile damage

Pewter's Raging trait adds conditional projectile damage at firing time:
`clamp((11 - health) / 10, 0, 1) * level * projectileDamageMultiplier`.
At 11 health or higher there is no bonus; at 1 health or lower it reaches its
maximum. The tool's material traits are rebuilt on a part swap. This implements
the projectile portion of [the original Raging definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/raging.json).

The single-player path uses the firing player's current health. The bonus is
temporarily omitted for LAN guests pending verification of the acting player's
health. Pewter is a compatibility material, and this change does not supply a
natural acquisition path. The melee portion remains separate work.

Rust syntax was checked; unit tests, the current-target build, and gameplay are
pending. Verification should compare both bows at health 20, 11, 6, and 1,
including multiple levels, definition stat multipliers, other damage modifiers,
rebuild and material replacement, save/reload, and LAN guest behavior. Label any
command-supplied material as a fixture.

## Raging melee integration pending

The same material's Raging melee module adds up to `2.5 * level *
attackDamageMultiplier` based on the attacker's current health, using the
same health factor as its projectile effect. The added damage precedes the
Jagged and Stonebound percentage traits, receives attack-cooldown scaling, and
is not itself multiplied by the critical bonus. The source module requires a
living target.

The shared Mixins manifest passes the target method’s `living: bool` to the
helper. The combined plan, build and runtime ordering still require verification
before this behavior is considered complete. The acting-player health path is
used for local and guest turns; melee gameplay and LAN remain unverified. Pewter remains a compatibility
material; the source alloy requires Tin or Lead, neither supplied by the base
TConstruct content.

Test both living and nonliving targets, health 20/11/6/1, broken tools,
multiple levels and non-unit tool multipliers, partial/full cooldown, critical
and secondary hits, Jagged/Stonebound combinations, material replacement and
LAN. Unit, current-target build and gameplay verification remain pending.
