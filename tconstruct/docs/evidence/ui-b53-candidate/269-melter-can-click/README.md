# Melter gauge Copper Can click

Same limited #263/#265/#269 integration, client SHA and isolated corrections as [the display comparison](../269-isolated-melter/README.md). This is not a whole exact-head build.

In a copied normal-tick world whose Melter held molten copper, the GUI drew its liquid. With an empty Copper Can on the cursor, a left click at (100,60) relative to the menu origin, inside the Melter gauge, left the can empty. The can remained present and the saved Melter still had 180 mB molten copper. In a separate fresh survival control, a Seared Fuel Tank supplied from a molten-copper bucket filled the same kind of empty can with `fluid:"tconstruct:molten_copper"`; logs contain the item components. Images are actual client captures. This identifies a bounded Melter gauge transfer problem, not a loss of items or fluid.

Static inspection: `source-smeltery-pointer.rs` still calls `tconstruct_source_machine_remote::position(game)`, which reads the transient use target, whereas #265/#269 preserve a separate remote screen position for drawing and tooltip. This is a likely cause to verify, not a proven runtime trace. It is separate from the tested display/tooltip fixes and remaining PR-wide gates.
