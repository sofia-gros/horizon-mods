# Ranged material traits (planned)

This unit targets source effects on the existing Longbow and Crossbow paths. It is not yet implemented or verified.

| Trait | Source ranged stat effect per level |
| --- | --- |
| Lightweight | +0.03 draw speed and +0.03 velocity |
| Heavy | +0.10 accuracy |
| Featherweight | +0.05 draw speed and +0.05 accuracy |
| Entangled | +0.15 draw speed, once |
| Vintage | multiply total draw speed by `1 - 0.1 × level` |
| Crystalbound | +0.10 velocity for ranged tools |
| Ductile | +0.25 projectile damage |

These are selected stat modules, not the complete modifiers. Entangled's unequip damage, Crystalbound's angle restriction, armor and movement effects, ammunition-specific effects and unsupported source tools remain open. Source: TConstruct revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, `ModifierProvider.java`.
