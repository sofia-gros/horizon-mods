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
