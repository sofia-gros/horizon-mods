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
server and every possible container action are not yet validated. The Part
Builder uses source-derived pattern selection, and the Tinker Station uses
source-derived layouts for supported three-part tools. Both use previews and
authoritative result transactions. Some click modes, detailed information and
the naming UI remain incomplete.

## Workstation interface improvements

This work covers the existing Part Builder and Tinker Station interfaces. It
aims to match their source layouts and images while keeping drawn slots, pointer
targets and synchronized container slots aligned. The Part Builder needs its
part cost and material information and its nearby inventory panel. The Tinker
Station needs the source selection layout, supported two- and three-input
layouts, a tool slot with three repair inputs, tool naming and information
panels. The same ordinary actions must work for the host and LAN guests.

Completion requires representative source-asset and coordinate checks, plus
gameplay checks for selection, inputs, naming, information, nearby storage and
existing crafting, repair and upgrade transactions. Four-input tools belong to
the later workstation that supports them; displaying an inoperable layout is
not completion. The smeltery and foundry interfaces are separate work. When an
original screenshot is unavailable, comparisons use the source and assets.
The adjacent inventory panel uses `textures/gui/generic.png` from the same
upstream revision as the other station textures; the resource test checks its
hash and includes it in the prepared GUI atlas.
The Part Builder information panel uses the upstream `panel.png`. Its displayed
recipe cost and input value come from the active recipe data, including on LAN.
For supported material parts, the same data frame also carries the selected
part's source material statistics and traits. Complete recipe descriptions
still need to be connected; the panel is not yet a complete original
implementation.
The Station's panels show the selected source layout and components, or the
current result's recorded durability, melee values, and supported upgrade
progress. Other tool statistics and modifier descriptions remain incomplete.
