# Station implementation: unresolved integration

This branch does not implement a Part Builder or Tinker Station. A correct
implementation needs persistent block inventories, host-authoritative result
transactions and client rendering/input in both single-player paths and LAN.
Closing a GUI, retrying a click and handling two viewers must not duplicate or
lose inputs. Tool repairs must retain unrelated item components and names.

At the pinned Horizon revision, the existing workstation UI is represented by
`game::stations::Kind`, `Station`, `Game<'a>::station`, and the vanilla menu
engine. `Game<'a>` owns both client and integrated-server state. The Mixins
0.5 state feature targets named nongeneric structs, and does not add enum
variants or automatically serialize new world data. A direct attempt to add
state to `Game<'_>` failed planning with `E_BRIDGE_TARGET`; this is a limitation
of that proposed integration, not proof that the complete station is
impossible. Mixins support modules and ordinary hook code may permit another
design, but that alternative and its save, authority and coexistence behavior
have not been established.

Do not implement the stations as a process-global UI inventory or as a
client-side preview that consumes items independently: neither would satisfy
world persistence or two-player atomicity. The next step is to design and test
a Mixins-only host-authoritative storage/transaction path and its network
projection before adding the menu and real station block assets.
