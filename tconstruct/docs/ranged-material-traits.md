# Ranged material traits (in progress)

This unit connects source effects to the existing Longbow and Crossbow paths. The combined stat and firing arithmetic has lightweight tests; actual game behavior is not yet verified.

| Trait | Source ranged stat effect per level |
| --- | --- |
| Lightweight | +0.03 draw speed and +0.03 velocity |
| Heavy | +0.10 accuracy |
| Featherweight | +0.05 draw speed and +0.05 accuracy |
| Entangled | +0.15 draw speed, once |
| Vintage | multiply total draw speed by `1 - 0.1 × level` |
| Crystalbound | +0.10 velocity for ranged tools |
| Ductile | +0.25 projectile damage |

The source material data assigns these traits to Cobalt, Lead, Aluminum, Twisting Vine, Ancient, Amethyst Bronze (ranged parts) and Steel respectively. Some are compatibility or later-game materials; their natural acquisition is not established by this change.

These are selected stat modules, not the complete modifiers. Entangled's unequip damage, Crystalbound's angle restriction, armor and movement effects, ammunition-specific effects and unsupported source tools remain open. Source: TConstruct revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, `ModifierProvider.java`.

Assembly and part replacement rebuild draw speed, velocity, accuracy and projectile damage from the material base and source trait levels. Existing Quick Charge, Trueshot, Diamond, Emerald and Netherite are applied in the same stat calculation; the resulting values feed draw timing and arrow creation. Existing saved bows must be rebuilt before these new values are present. Conversion, build, save and LAN tests remain pending.
