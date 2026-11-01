# Material armor damage and repair

This draft targets material armor damage mitigation, per-slot durability,
broken-item retention and disabled broken attributes, and restoration after
Station repair. Existing Horizon damage-source, armor, enchantment and LAN
paths remain authoritative. Registered items and displayed attributes alone do
not demonstrate these actions.

The representative acceptance plan covers multiple material armor slots,
armor/toughness and Ductile, fire and armor-bypass damage, last durability and
broken retention, repair/re-equip/save, hosted LAN guest, and vanilla armor
wear/break regression. Other source armor modifiers and equipment effects,
projectile firework stats, PvP area attacks, shears loot and trim remain open.

The current draft captures the existing `durability_loss` result during
`hurt_armor` after the shared Dense adjustment, then applies it to matching
unchanged source armor slots that have per-item material durability and no
fixed registry maximum. It uses the existing retained-damage helper; an
item that breaks has equipment attributes recomputed. Source repair lookup
now includes repairable armor plating, material-stat definitions, and fixed
material / statless-part repair modules. Registered source armor currently has
no registry maximum; fixed-max source armor is intentionally left on the
native path rather than being damaged twice. Protection modifiers and other
armor-specific abilities are not implemented by this change.

The scoped official Mixins plan accepts the armor hooks, existing Dense hook,
module and state. Two Rust tests checked mapping identical/mixed slots and
ignoring a replaced item; Python suites passed. Formal Rust type checking,
full transformation, target builds, and game/LAN tests have not run. Original
repair rates and damage-source interactions still need representative in-game
validation; no behavior is claimed complete on planner acceptance alone.
