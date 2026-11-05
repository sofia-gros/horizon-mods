### Reinforcement casting outputs

Iron, Gold, Cobalt and Obsidian Reinforcement now have native item declarations
and manual resource-overlay entries. Manual entries are appended after
Tinkers' Gadgetry, preserving earlier protocol IDs. The fixed source casting
recipes consume one Pattern: iron 90 mB / 60 ticks, gold 90 mB / 57 ticks,
cobalt 90 mB / 65 ticks, and obsidian 1,000 mB / 221 ticks.

The bounded Rust casting fixture checks catalog selection, unregistered output
rejection, wrong machine/input/fluid rejection, partial fills, cooling, fluid
consumption and Pattern consumption. Python checks pin the recipe fixtures,
item/model/texture declarations, and generated manual resources. This does not
claim modifier application/effects, actual host casting, Creative placement,
save/reload or LAN validation.
