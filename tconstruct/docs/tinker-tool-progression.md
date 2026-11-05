# Tinker Tool advancement ancestry

The source Tinker Tool goal is restored under Tinker Station, making the
existing Tool Smith challenge's parent chain complete. Its inventory condition
uses the current world's `tconstruct:modifiable/multipart` tag, represented in
the host's native `items: "#tag"` predicate form. This does not establish that
every multipart tool is obtainable or implements its full effects.

Both the host inventory candidate index and its item-tag predicate read the
same resolved world snapshot for this tag and the bounded recipe-unlock tags
listed in [tagged recipe unlocks](tagged-recipe-unlocks.md). Other tag predicates
retain native behavior. Missing, invalid or empty snapshots match nothing; native
resource membership is never used as a fallback. Snapshot refresh invalidates
the cached candidate index. It does not synthesize an inventory-change event:
the host hashes item IDs, counts and slots and skips unchanged inventories.
After a tag-only reload, an already-held newly eligible tool is evaluated on
the next inventory change, not immediately. The source goal also uses an
inventory-change criterion, not a tick criterion; this patch deliberately does
not broaden event triggering or revoke already-earned progress. Exact upstream
reload timing has not been established. The authoritative host shares that index while
executing guest actions; actual LAN behavior remains unverified.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`src/generated/resources/data/tconstruct/advancements/tools/tinker_tool.json`.
The original icon's display-only NBT is omitted because the host icon parser
supports only the item ID. Parent, title, description, requirements, telemetry,
notification flags and lack of rewards are preserved.

Tests cover source/native criterion mapping and parent ancestry, snapshot
membership versus stale native tags, unrelated tag fallback, cache reuse,
refresh invalidation and missing/invalid snapshots. The isolated Rust fixture
models host index construction; it is not a complete advancement-engine test.
Mixins planner acceptance is checked separately. Actual tree layout, inventory
events, toast/chat, save/reload and LAN are unverified. Real-machine testing is
excluded from the v0.5 completion condition. Other missing advancement parents
and general legacy tag compatibility remain separate work.
