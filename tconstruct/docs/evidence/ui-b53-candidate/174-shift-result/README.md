# Station repair result shift-click observation

Same combined candidate, client SHA, #263 content and isolated corrections as [the Flint and Brick fixture](../174-flint-and-brick/README.md). Not an unmodified exact-head build.

With a valid Flint and Brick / Seared Brick repair preview and empty cursor and inventory space, shift-clicking the repair result did not transfer it. The tool, brick and result remained present, and the cursor remained empty. The harness expected a transfer and ended with `StopIteration`; its failure is retained in the log. The candidate's `station-pointer.rs` and `part-output.rs` explicitly suppress the Station QuickMove path. This is an independently unresolved Station UI operation, not evidence of lost inputs or that #174's ordinary-click repair is incorrect; source behavior and full implementation need separate review. Screenshot is from the actual client.

Source follow-up: [TinkerStationContainerMenu.quickMoveStack at pinned upstream f4e77d4](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tables/menu/TinkerStationContainerMenu.java#L117) explicitly handles result shift-click and player-inventory insertion. Thus suppression is an original-parity gap; transaction and full-inventory behavior still need a safe implementation and verification.
