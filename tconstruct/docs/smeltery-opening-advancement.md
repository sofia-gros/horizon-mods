# Successful Smeltery menu opening progression

Adds source `tconstruct:smeltery/structure` below Mighty Smelting. The source
criterion remains `open_smeltery`, trigger `tconstruct:block_container_opened`,
and block-entity type `tconstruct:smeltery`. Only the display icon's `item`
key becomes native `id`; task frame, translations, toast enabled/chat
disabled, visibility and no-rewards behavior are unchanged. The existing
Tinkers' Anvil and Tool Forge goals now have their missing ancestor resource.

The bounded trigger adapter runs from the successful Smeltery menu branches:

- Local opening follows machine initialization, current-world structure
  validation, readable Smeltery state, nonempty slots and container assignment.
  Temporary guest acting contexts are excluded here, so a preparatory open
  cannot award before the network menu is actually created.
- Guest opening follows the existing structure/state validation and network
  menu creation. The guest must still have a menu for the same position and
  block slots. The trigger then runs through `as_guest` for that connection,
  never directly against the host's player progress.

The helper maps `tconstruct:smeltery_controller` to its exact source
block-entity type. The separate Foundry opening extension also maps
`tconstruct:foundry_controller` to `tconstruct:foundry`, without allowing one
to satisfy the other. It evaluates the source player predicate with
the host advancement predicate machinery, then uses the normal event pipeline
for criterion tracking, completion, notifications, rewards and guest packets.
It does not directly grant hard-coded advancement IDs. Remote clients do not
award locally. Reopening uses the host's existing completed-criterion guard.

This does not register every possible block-container trigger type,
placement/possession substitutes, or new machine behavior. The Foundry
opening extension is documented separately, including its missing Alloyer
ancestor. Existing menu permission checks and structure rules are unchanged.
The hook records the established authoritative menu-open boundary; network
delivery and actual display have not been exercised.

Exact advancement and Java trigger source, SHA-256 paths and attribution are
in `tests/fixtures/smeltery_opening/`, from Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`.

Brief static review only. Added Python source/ancestry/wiring/package checks
and a Rust production-helper fixture for exact types, actor predicates,
unrelated blocks, client suppression and repeated criteria. The Rust fixture
uses a small Game/advancement adapter, not the full host or LAN transport.
Tests were not run. No builds, CI inspection, version checks or merges.
Mixin application, native typing, actual menu opens, dedicated server/LAN
actor attribution, toast/tree display and saved progress remain unverified.
