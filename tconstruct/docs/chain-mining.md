# Vein Hammer and Broad Axe mining

This draft implements the source Vein Hammer's same-block, distance-two vein
traversal and Broad Axe's source tree and non-tree traversal. It extends the
existing authoritative per-block break, drops, wear and spawn-protection paths.
Creative supplies these four-part tools for now; survival assembly remains a
v0.5 progression requirement. Expanded modifiers are not included.

Implementation and game verification are pending.

The current implementation uses the source six-direction, same-block BFS with a
maximum distance of two for Vein Hammer. Broad Axe follows the source's
one-column trunk and bounded branch traversal for blocks in the source tree-log
tag; other effective blocks use the source upward box. These iterators feed the
existing player break path. Spawn protection is checked with the existing server
policy for all candidate columns, including branches beyond the original 3×3.

Lightweight tests cover traversal ordering, unsupported branches, and protection
at a distant boundary. A source-tag fixture checks vanilla tags remain unchanged.
The Mixins plan is accepted; Rust type checking and in-game behavior remain
unverified. Validate same/different/diagonal ores, distance limits, branching
logs and obstructed branches, non-log targets, partial tool wear/breakage,
actual drops, protection boundaries, save/reload, and LAN with the same build.
