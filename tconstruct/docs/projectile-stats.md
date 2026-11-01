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
