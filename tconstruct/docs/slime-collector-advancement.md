# Slime Collector acquisition advancement

Adds the source `tconstruct:world/slime_collector` task beneath the existing
Tinkers' Gadgetry parent. Its ancestry resources connect to Materials and
You; actual tree display remains unverified.

The five independent source criteria are Earth, Sky, Ichor, Ender and Magma
Cream. Each is required, and each can be earned on a different inventory
event. Earth/Sky/Ichor/Ender use the source `forge:slimeball/<kind>` tags.
Magma Cream is the direct `minecraft:magma_cream` item criterion, not a fifth
slime tag. The source Ichor Slime Ball icon, translated title/description,
visible task frame, toast enabled and chat announcement disabled are kept.
There are no rewards and no requirement to possess all five simultaneously.

The only adaptations are the four legacy tag predicates to native
`items: "#tag"` and icon `item` to `id`. The existing bounded advancement
bridge already manages all four tags, so no runtime hooks or tag changes
are added. Both inventory indexing and final matching use current-world
membership. Missing/invalid/empty snapshots do not fall back to stale
native members; tag reload invalidates the cache but does not synthesize
inventory events or revoke prior progress.

This adds acquisition tracking, not new slimeball recipes, natural drops,
slime entities, islands, physics or item effects. It does not establish that
every source resource is naturally obtainable on the current host.

Exact source bytes, SHA-256 and attribution are in
`tests/fixtures/slime_collector/`, from Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`.
Source/translation, five separate requirements, ancestry, package and
translation regression checks were added but not run. Brief static review
only; no builds, CI inspection, version checks or merges. Actual acquisition,
inventory events, notifications/tree display, save/reload and LAN remain
unverified.
