# Station implementation

The two custom blocks are built into a separate resource root by the pinned
Horizon data-table builder, invoked from a Mixins-injected native entry point.
The generated gameplay facts mark their block states as having block entities.
A Mixins hook opens an existing per-world `BlockEntity::Chest` storage at each
table position. Its normal chunk save/load path persists inputs; an injected
`pre_remove` mapping makes normal block removal spill the stored stacks.
There is no process-global station inventory. The original textures and block
model are attributed in `assets/tconstruct/NOTICE.md`.

Both the local-player click path and the LAN server's container-click path
invoke the same transaction code against current authoritative inputs. It
validates all inputs and cursor capacity before changing any slot, moves or
clones the result once, and marks the chunk modified. The server resends the
current menu after a result attempt so a losing viewer cannot keep a stale
preview. Repair clones the existing tool and changes its damage, retaining its
name and custom data. Separate Mixins carry that tool's custom data across
Horizon network stacks; arbitrary other components remain outside this slice.

The previous attempt to add state directly to `Game<'_>` failed Mixins planning
with `E_BRIDGE_TARGET`. The current implementation avoids that restriction by
using existing per-world block storage and per-player cursor state. Dedicated
server and every possible container action are not yet validated. The GUI
supports ordinary pickup; shift-click, drag and pickup-all are deliberately
blocked to avoid bypassing the transaction. Three fixed part selections now
use a preview and the separate result slot, with per-world selection state and
a virtual menu synchronization slot. Recipe-derived selection, complete
material variants, detailed material/trait tooltips and naming UI remain future work.
