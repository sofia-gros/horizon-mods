# Composite material casting

A composite recipe pours fluid over an existing material part and consumes that part to make the same kind of part with a different material. It is separate from pouring metal into a reusable or single-use cast. The source recipe sets the input material, output material, fluid amount per part cost and temperature; the result retains its output material identity.

For example, a copper pick head and 500 mB of water match the original oxidized-copper recipe. A wood tool handle and 250 mB of molten obsidian match the original Nahuatl recipe. A string bowstring and 90 mB of molten Rose Gold match the source Rose Gold recipe, provided that material is supported for the item. The retained original composite recipes do not include a Cobalt-input transformation; Cobalt parts are not claimed to gain an additional composite recipe. These examples describe the source recipes and new lookup; this change has **not yet been tested in the game**. Availability also depends on the actual source fluid, item, material and tag registries.

The lookup checks material variants, input and output part compatibility, recipe conditions, material predicates, and shield casting conflicts. It uses the existing reserved-fluid and cooling transaction, with one input part and an empty output slot. Unknown predicate forms and incomplete registry data are rejected. Automated lookup and transaction tests cover a source copper recipe, variants, invalid input, fluid consumption, output occupancy and save/reload of an in-progress transaction. They do not establish survival, LAN or game rendering behavior.

![A Melter in the existing Linux playtest](melter-filled.png)

This image shows a previously tested Melter, not a composite casting test. See [Melting and casting](melter-casting.md) for the existing equipment setup.
