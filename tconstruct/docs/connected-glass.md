# Connected glass

> **Provisional build:** Some effects and operations described below are unavailable or under verification. Earlier test results refer to their stated revisions; see [current limitations](provisional.md).

Connected glass joins its texture to matching neighboring blocks. You can find clear, colored, seared, and scorched glass and panes in the **Tinkers' Construct Blocks** Creative tab.

![Clear glass joined next to colored and seared glass](connected-glass.png)

*The two clear blocks on the right join into one surface. Different glass types keep their own borders.*

Place panes next to each other to make a window. Their shape changes with their neighbors, and stacked panes keep the exposed edge when their connections differ.

![A pane edge visible beside a narrower pane above it](connected-pane.png)

*The lower pane has an arm that the pane above it does not; its exposed top remains visible.*

Soul Glass can be walked through and slows movement. On Linux, representative glass and panes have been checked for Creative acquisition, placement, connected appearance, pane shapes, breaking, saving, and LAN synchronization. The full set has not been tested individually. Soul Glass's special view-blocking behavior and Windows play have not been verified.

Smeltery behavior, material-dependent anvils, and custom storage are separate work. See the [block guide](block-registration.md) for the rest of the registered building blocks.

## Clear glass panes

The pinned source recipe crafts 16 Clear Glass Panes from six Clear Glass blocks in two rows of three. Source, registration and recipe-collision checks pass. The source data also contains a sand-to-molten-glass and basin-casting route to the input, but actual acquisition, crafting, recipe-book display, save/reload and LAN remain unverified.
