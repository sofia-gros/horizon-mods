# Mining, durability and repair trait follow-up

This draft extends material traits from original Tinkers' Construct revision
`f4e77d43` using shared calculation and the existing mining, wear and repair
paths. The intended mining modules are Lightweight, Heavy, Vintage, Entangled,
Jagged, Stonebound and Maintained. Dense is included if its source repair and
wear formulas can use the existing authoritative transaction paths. Overlord
requires functional Overslime and is not represented as completed here.

Acceptance covers original values and operation order, material levels,
effective versus ineffective blocks, undamaged/part-worn/nearly broken tools,
part replacement, save/reload, and representative single-player and LAN
mining/repair. Vanilla tools and existing Preserved/Ductile/Cultivated paths
need representative regressions. Registered material data alone does not mean
a modifier's other modules are implemented. No Mixins transform, build, game
validation or CI has been performed for this draft.
