# Material projectile speed and accuracy

This draft targets the original Longbow and Crossbow projectile speed and
accuracy using their saved material stats. The source bow multiplies charge by
velocity; crossbows multiply their projectile speed by velocity. Both use
`3 * (1 / accuracy - 1)` for inaccuracy. Existing Horizon random generation is
retained. A registered stat or a successful planner probe does not demonstrate
that the emitted arrow uses it.

Representative Linux gameplay using runtime `f898d875dad20b4a3d3752a75b910258197a2fab` confirmed actual arrow speed and spread for both weapons, source fractional low/partial/full charge, main/offhand, source three-arrow fan, ammunition, mob hits, survival wear and broken retention, repair and save/reload, hosted LAN guest shots, and vanilla bow/crossbow firing. A Tinker Station swap from wood to cobalt limb changed Crossbow `tic_velocity` from 1.0 to 1.05 and measured arrow Z velocity from approximately 2.838 to 2.984; the replacement persisted after reload. Several comparisons used synthetic stat data to isolate the shared formula, and are not a test of every material. The speed samples include normal flight drag.

Official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab` applied in 198.89 seconds; the client/ctl build succeeded in 1m24s. Four scoped Rust tests, 44 TConstruct Python tests and 7 root Python tests passed. Windows and material combinations beyond the representative cases are unverified. Firework rockets, conditional stat modifiers, armor actions, PvP area attacks, crop shears loot and trim remain unfinished.

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
dispatch threshold; the representative low-charge case passed; conditional stat modifiers remain
unimplemented.
