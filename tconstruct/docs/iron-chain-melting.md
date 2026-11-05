# Native Iron Chain melting

The pinned TConstruct source recipe matches `minecraft:chain`, but the pinned
Horizon host records the vanilla rename to `minecraft:iron_chain` in
`crates/horizon-world/src/datafix/fixes_c.rs`. The native item therefore could
not match the source recipe despite the existing melting runtime.

Resource preparation now changes only the ingredient's item ID in the generated
`smeltery/melting/metal/iron/chain` source recipe. It requires a native Iron Chain
registration and no legacy Chain registration. The original result remains
110 mB of `forge:molten_iron`, temperature 800, and source time value 66. No ore
multiplier, fuel behavior or machine timing is changed.

Both bundled startup and manual preparation call the same helper. Owned startup
caches are invalidated. Existing manual resource outputs must be regenerated
with `--tconstruct-prepare` into a fresh output directory; player-owned resources
and worlds are not rewritten. Pinned archives stay unchanged and enabled world
datapacks still override the generated source layer. Copper Chain does not match.

Tests use the byte-identical pinned recipe with the production melting parser
and ingredient matcher. They cover the native match, rejected legacy/copper
matches, unchanged yield/heat/time, condition handling, registry gating, exact
field updates, repeated preparation and unrelated file preservation. This does
not claim live furnace interaction, world save/reload, or multiplayer testing.
Chain-dependent modifiers and hanging signs are outside this bounded runtime fix.
