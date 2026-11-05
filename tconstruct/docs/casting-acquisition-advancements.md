# Sand and Gold Casting acquisition advancements

Adds the source `tconstruct:smeltery/sand_casting` and
`tconstruct:smeltery/gold_casting` advancements below the existing Melter
parent. This also supplies the missing parent of the already packaged
Cast Collector goal. The ancestry resources now connect through Melter,
Puny Smelting and Materials and You; actual tree display remains unverified.

The source Sand Casting criterion is possession of a member of
`tconstruct:casts/single_use/blank`, not every shaped sand cast. Its default
members are Blank Sand Cast and Blank Red Sand Cast. The source display icon
is nevertheless the Tool Handle Sand Cast. Gold Casting accepts the source
`tconstruct:casts/gold` tag and uses the Pick Head Gold Cast icon.
Both retain `crafted_cast`, the inventory-change trigger, source parent,
single requirement, translated title/description, visible task frame and
toast enabled/chat announcement disabled. Neither grants recipe or item
rewards, and neither changes the recipes or cast registrations.

Only the legacy inventory `tag` predicate and icon `item` key are translated
to native `items: "#tag"` and `id`. Both cast tags are added to the existing
bounded current-world bridge, stacked on PR #814. The inventory candidate
index and final predicate use the same resolved world snapshot, preserving
datapack replacement rather than expanding a fixed list of casts. Missing,
invalid and empty managed snapshots match nothing, even if the native
resource table has stale members. Reload invalidates the inventory index
but does not synthesize an inventory event or revoke earned progress.
An already-held newly eligible cast is evaluated on the next inventory event.

Exact source fixtures, paths and SHA-256 hashes are in
`tests/fixtures/casting_acquisition/`. Source: Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`src/generated/resources/data/tconstruct/advancements/smeltery/`.

Regression coverage was added for source translation, ancestry, packaged
goals/tags/icons/members and both tags in the existing replacement, reload,
missing/invalid/empty snapshot and stale-native fallback fixture matrix.
Tests were not executed. Brief static review only; no builds, CI inspection,
version checks or merges. Gameplay acquisition, toast/tree display, actual
inventory events, save/reload and LAN remain unverified.
