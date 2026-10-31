# GUI alignment work

This draft covers the existing Part Builder, Tinker Station, Melter, Smeltery and Foundry screens. The original Tinkers' Construct source and assets are the reference for each screen.

For each existing screen, compare its background and sprite crops, slot positions and item rendering, input hit areas, tooltips, text, scrolling and tabs. Check the same operations at 960 × 540 and 1280 × 720, including a representative LAN session. The Tinker Station result item currently appears outside its result frame and needs a specific fix.

This is a work checklist, not a statement that the screens already match. New machine functionality and screens for unimplemented features are outside this change.

## Source comparison

- Station result: the older upgrade screenshot used a separate result coordinate for repairs and upgrades. The original has one result slot at (114, 38); the current layout maps both operations there. Verify rendering, clicking and tooltip in a fresh game before replacing the screenshot.
- Smeltery and Foundry: the original heating screen supplies an empty-bucket hint, a mode-button hover and a three-line mode tooltip. These are being connected to the existing screen; game verification is pending.
- Adjacent table tabs use the source six-face scan and order. Part Builder and Tinker Station switches are being connected for local and LAN play; registered tables without menus remain visibly unavailable. Game verification is pending.
- Part Builder, Melter and the remaining screen interactions still need source and runtime comparison.
- Station armor and offhand frames correspond to five source inventory slots. A local/LAN path using the existing inventory transaction rules is in progress; test equipment placement, removal, Shift-click and cursor preservation before claiming support.
- Melter and heating structures use original gauge and heat-bar sprites, fluid hover regions and status text. Burn duration is saved for the source flame indicator; older saves use the remaining burn time as their first displayed total. Runtime and LAN verification are pending.

Still to verify: source fuel-area tooltips and fuel-tank contents, text and click boundaries at both screen sizes, all visible tab states, Part Builder patterns and side inventory, and heat-bar/tooltip behavior while recipes heat or cannot heat. Compare against the source screen code and assets instead of using earlier screenshots as a reference.
