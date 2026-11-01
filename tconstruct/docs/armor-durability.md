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

The implementation captures the existing `durability_loss` result during
`hurt_armor` after the shared Dense adjustment, then applies it to matching
unchanged source armor slots that have per-item material durability and no
fixed registry maximum. It uses the existing retained-damage helper; an
item that breaks has equipment attributes recomputed. Source repair lookup
now includes repairable armor plating, material-stat definitions, and fixed
material / statless-part repair modules. Registered source armor currently has
no registry maximum; fixed-max source armor is intentionally left on the
native path rather than being damaged twice. Protection modifiers and other
armor-specific abilities are not implemented by this change.

Official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab` transformed runtime `7fb00d0fb7f9ccda0d13df3971d03fcc1aa05d14` in 197.39 seconds; the Linux client/ctl build succeeded in 20.32 seconds. Two Rust slot-mapping tests and the 44 TConstruct plus 7 root Python tests passed.

Representative Linux game tests confirmed the first hit uses the equipped armor and toughness before a last-durability helmet breaks; the next hit uses only the remaining chestplate. Both items consumed one point, the helmet stayed equipped, and an iron Station repair restored its armor and toughness. Save/reload, a steel-plating replacement with Ductile toughness, Traveler's Helmet statless leather repair, Slime Wings fixed phantom repair, a fire-resistant slot, magic armor bypass, hosted LAN guest break/sync, and vanilla iron armor break were also checked. Some items used synthetic material data to isolate the behavior. These are representative cases, not all armor families, modifiers, damage types or platforms; Windows remains unverified.
