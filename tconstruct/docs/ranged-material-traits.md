# Ranged material traits (in progress)

This unit connects source effects to the existing Longbow and Crossbow paths. The combined stat and firing arithmetic has lightweight tests; actual game behavior is not yet verified.

| Trait | Source ranged stat effect per level |
| --- | --- |
| Lightweight | +0.03 draw speed and +0.03 velocity |
| Heavy | +0.10 accuracy |
| Featherweight | +0.05 draw speed and +0.05 accuracy |
| Entangled | +0.15 draw speed, once |
| Vintage | multiply total draw speed by `1 - 0.1 × level` |
| Crystalbound | +0.10 velocity for ranged tools and quantized horizontal arrow launch angle |
| Ductile | +0.25 projectile damage |

The source material data assigns these traits to Cobalt, Lead, Aluminum, Twisting Vine, Ancient, Amethyst Bronze (ranged parts) and Steel respectively. Some are compatibility or later-game materials; their natural acquisition is not established by this change.

These are selected modules, not the complete modifiers. Entangled damages the holder on actual equipment removal, including while not drawing; releasing an arrow alone is not removal. The pinned host's main-hand hotbar selection and inventory replacement paths do not share the existing armor/offhand equipment callback, so the authoritative local and LAN unequip paths still need a common safe hook. Crystalbound's sling and knockback angle restrictions, armor and movement effects, ammunition-specific effects and unsupported source tools remain open. Source: TConstruct revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, `ModifierProvider.java`, `DamageOnUnequipModule.java` and `RestrictAngleModule.java`.

Assembly and part replacement rebuild draw speed, velocity, accuracy and projectile damage from the material base and source trait levels. Existing Quick Charge, Trueshot, Diamond, Emerald and Netherite are applied in the same stat calculation; the resulting values feed draw timing and arrow creation. Existing saved bows must be rebuilt before these new values are present. Conversion, build, save and LAN tests remain pending.
