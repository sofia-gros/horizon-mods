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

## Draft implementation and limits

Station and Creative regeneration store the original harvest-tagged tool's
Lightweight/Heavy/Vintage/Entangled stat and ordered Jagged/Stonebound/Maintained
levels on the item. A shared item boundary recognizes effective per-stack tool
rules for both the client and host; separate context hooks apply conditional
speed after mining efficiency and potion effects, before underwater/airborne
penalties. Materials and part swaps rebuild the stored values. Previously saved
tools need regeneration before these new fields are present.

Dense repair uses the original level ranges. Dense damage prevention rolls once
per durability point through the existing shared `durability_loss` path.
Original conditional wear formulas use the `multiply_all` durability multiplier;
the earlier melee calculation mistakenly included material percent and
`multiply_base` bonuses. The draft corrects that multiplier for the existing
Jagged/Stonebound melee path too and needs representative regression checks.

The latest official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab`
accepted an isolated plan for the shared item RETURN, host/client speed stores,
and Dense durability RETURN. This is **not** compilation or gameplay proof.
Lightweight unit checks cover source formulas, insertion order, effective versus
ineffective rule matching, broken tools, stat rebuild, and deterministic Dense
rolls. The exact generated hooks, tag loading, client/host agreement, potion and
efficiency order, save/reload, LAN, and all new gameplay still require the
authorized later transform/build and representative game checks. Applied
modifier mining effects not listed above and all other trait modules remain
incomplete. Overlord/Overslime, Windows, and every-material coverage are not
claimed here.
