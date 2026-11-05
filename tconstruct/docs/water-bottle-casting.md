### Water bottle table casting

The three fixed-source water-bottle Casting Table recipes now support the
result NBT `{Potion: "minecraft:water"}` for normal, splash and lingering
potions. They consume one matching empty bottle and 500 mB from the resolved
`mantle:water` fluid tag, then cool for one tick. The world recipe/tag snapshot
and existing casting transaction remain authoritative.

Only this exact result metadata is supported: other potion types, extra NBT,
result counts and NBT on other items remain rejected. A plain potion result
and a water-potion result are distinct during conflict detection. The native
potion component is created before fluid reservation and persists through the
existing casting state codec. This does not implement arbitrary result NBT,
Ancient Repair Kit casting, or additional potion effects.

The source fixtures, catalog checks and machine state roundtrips pass in the
bounded Rust harness. Actual host crafting, rendering, world reload and LAN
remain unverified.
