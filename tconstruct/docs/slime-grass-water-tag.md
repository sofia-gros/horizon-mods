# Slime grass spread and the water tag

Pinned TConstruct f4e77d43 checks the live `minecraft:water` fluid tag before
spreading slime grass. The port previously hard-coded Water and FlowingWater.
The adapter now consults the world-scoped, resolved source fluid tag snapshot.
If the whole resolved snapshot fails, it resolves the water tag alone against
the host-supported fluid kinds, so an unrelated tag cycle does not stop spread.
A required water-tag entry outside those kinds makes full resolution fail.
The fallback still permits a fluid proven absent from the raw tag graph, such
as ordinary air; a potentially matching fluid remains unresolved and is skipped.
A missing or malformed water tag, or an unresolved potentially matching fluid,
causes that attempt to be skipped rather than guessed. The ordinary validity check, nylium's
non-spreading ticks and vanilla grass remain unchanged.

Standalone tests cover separate tag snapshots, additions, removals, absent
snapshots, unrelated tag cycles, an air/empty-fluid target, and absent or cyclic
water tags, and an unknown required fluid with air as a proven non-member,
plus static adapter wiring. The current host supports
only its registered fluid kinds. The legacy source-pack tag capture, first-tick
readiness and reload have not been exercised with live worlds or LAN; the host
calls ensure_datapacks in tick_functions before level ticks, but failed or
missing snapshots deliberately suppress spread. This change does not implement
custom world-fluid registration or claim complete datapack parity.
