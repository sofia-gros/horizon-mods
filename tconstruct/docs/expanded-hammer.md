# Expanded Sledge Hammer

Addresses high-priority #379 Expanded acquisition and mining-area behavior for
Sledge Hammer only. Pinned TConstruct f4e77d43 supplies the five-input ability
recipe: two Pistons, one Amethyst Bronze ingot and two Ichor Slimeballs. Each level
uses one ability slot. Five ordinary inputs require an Anvil; source modifier
crystals use the existing single-input path. Extraction and ability accounting
use the existing upgrade mechanism, preserving unrelated tool data.

The supported canonical box starts at 3×3. Odd levels add one width radius, even
levels add one height radius, with source width-first traversal and the center
omitted from extra positions. Supported levels 0–8 range from 3×3 to 11×11. Levels
outside this bounded implementation and malformed saved values fail closed.
Fireprimer also contributes the source expansion flag; unsupported Fireprimer
combinations are rejected rather than silently undercounted.
Other tools cannot acquire Expanded through this implementation. Their existing
saved-value consumers are not advertised as newly complete.

Both Station planning and mining check current world modifier definition, tool
box definition, AOE tag and resolved traits. The shared helper is installed in
both game and client_lib crates. Unsupported/reloaded definitions do not activate
the expanded hammer path. The paid plan never overwrites malformed Expanded.

Every candidate still uses the existing authoritative player break method,
hardness/effectiveness checks and per-block tool durability checks. Expanded's
level is revalidated between blocks. Guest protection scans the same enlarged
geometry, including at the delayed-destroy boundary. Breaking the tool, changing
its expansion or a break error stops traversal; scoped guards are cleaned up.
The initial block's permission checks remain in the native break path.

Tests execute the actual area hook, world gate and recipe planner using real
ItemStack/NBT, source fixtures and narrow Game/world/material-resolution adapters.
The actual protection callback helper is tested separately with a permission
adapter. Native selection indices remain stable. Full host compilation, actual
GUI transaction, save/network/LAN and concurrent external callbacks remain
separate gates, not implied by these tests. No saved schema is introduced.
