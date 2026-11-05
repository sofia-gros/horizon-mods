# Gadgetry and Copper Gauge recipe discovery

Two existing crafting recipes now have the pinned source recipe-book unlocks:

- Tinkers' Gadgetry: inventory membership in `forge:slimeball/sky` OR an unlocked
  event for `tconstruct:tinkers_gadgetry`, rewarding that recipe.
- Copper Gauge: inventory membership in `forge:ingots/copper` OR an unlocked
  event for `tconstruct:copper_gauge`, rewarding that recipe. Its source recipe
  ID is `tconstruct:smeltery/seared/gauge`, not `smeltery/copper_gauge`.

Both reuse the existing bounded current-world tag bridge without adding tags
or changing runtime matching. Candidate indexing and final predicates share
the resolved snapshot, missing/invalid/empty membership fails closed, and
unrelated native tags remain untouched. A tag-only reload takes effect on the
next inventory event; earned progress is not revoked.

The source hidden parent, criterion names, OR requirements and single rewards
are preserved. Adaptations are flattened native recipe IDs, string-valued
`recipes` event conditions and native `items: "#tag"` predicates. No display
or achievement toast is added. The existing Gadgetry book-possession
advancement remains separate and still requires obtaining the actual book.

Recipes, counts, inputs, book reader/chapter limits and Copper Gauge placement,
support checks and tank interaction are unchanged. This does not establish
first Sky Slimeball acquisition, broaden crafting ingredient support or grant
new book chapters. Copper Gauge still yields four items; Gadgetry yields one.

Exact source fixtures, paths and hashes are in
`tests/fixtures/gadgetry_gauge_unlocks/`, pinned to Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. Regression coverage was added for
source criteria, mapping, existing recipes/registrations and packaging.
No tests or compilation were run; brief static review only. Actual events,
recipe-book UI, crafting, reload, persistence and LAN remain unverified.
No CI inspection, version checks or merge was performed.
