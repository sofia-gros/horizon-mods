# Direct fuel tank container use

Combined source cc868cdb includes #166 head aceaabd745dee5e292d251336bb64cefc045121e. This test used the later limited #263/#265/#269 integration, client SHA and isolated corrections documented in [the Melter fixture](../269-isolated-melter/README.md). It is not a full exact-head #166 build.

In fresh creative fixtures switched to survival, the Seared Fuel Tank accepted a water bucket, filled a glass bottle with a water potion, and accepted that potion back as an empty glass bottle. In a second fixture, a water-filled tank refused a lava bucket without consuming it; an empty bucket subsequently recovered water. Screenshots are actual client captures and the logs include held stacks. This is bounded local direct-use coverage; other containers, limits, full inventory, persistence, LAN and independent gates remain open.
