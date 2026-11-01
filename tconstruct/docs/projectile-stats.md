# Material projectile speed and accuracy

This draft targets the original Longbow and Crossbow projectile speed and
accuracy using their saved material stats. The source bow multiplies charge by
velocity; crossbows multiply their projectile speed by velocity. Both use
`3 * (1 / accuracy - 1)` for inaccuracy. Existing Horizon random generation is
retained. A registered stat or a successful planner probe does not demonstrate
that the emitted arrow uses it.

Acceptance will cover representative materials, arrow speed and spread,
charging, ammunition, hit, durability, repair, save/reload, hosted LAN and
vanilla regression. Armor actions, PvP area attacks, crop shears loot and trim
remain separate unfinished work. Formal transformation and game validation
have not yet run.

Official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab` rejected
`release_bow` MODIFY_ARG at `arrow.shoot_from_rotation` with
`E_UNSUPPORTED_CONTROL_FLOW` (closure context), and `shoot_crossbow`
MODIFY_ARG at `arrow.shoot` with the same code (reference temporary lifetime).
A separate `add_projectile` HEAD probe was accepted, but runs after random
spread has already been applied. It must not merely scale the noisy vector:
that would not reproduce source accuracy or multi-shot direction. A safe
shot context and source-direction/actor handoff must be validated before
connecting that boundary. Firework rockets use a separate launch path.
The source stat reader is a standalone prototype and is not wired to gameplay.
