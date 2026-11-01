# Material stat trait follow-up

This draft covers the existing source-derived material trait dispatcher and the melee stat portions of four source modifiers at Tinkers' Construct `f4e77d43`: Lightweight (attack speed), Solid (attack damage), Pierce (attack damage), and Vintage (attack speed). Use their original `stat_boost` operations and levels, the existing material stat calculation and combat attribute paths; recalculate from base values during assembly, Creative generation and part replacement, preserving applied upgrades and avoiding double application.

Acceptance requires representative material/level combinations, source operation order, actual attack damage and cooldown, part replacement, saving/reloading, a Station-upgrade interaction and an ordinary LAN transaction. Group source-data and calculation checks; do not repeat every tool/material combination. The build and game checks are pending.

Only these melee-stat portions are in scope. Mining speed, draw speed, projectile velocity/damage, protection, movement, Pierce's mob effect, Vintage's extra slots and other modifier modules are not implied to work. Existing Preserved, Ductile, Jagged, Stonebound, Cultivated and vanilla behavior require representative regression checks. No new GUI, world generation or PlutoLib API is included.
