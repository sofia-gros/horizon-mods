# Guide acquisition advancements

The two introductory guide acquisitions use the original TConstruct 1.20.1
advancements from revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`.
Obtaining Materials and You satisfies `tconstruct:tools/materials_and_you`;
obtaining Puny Smelting satisfies its child `tconstruct:smeltery/puny_smelting`.
Both use the source `minecraft:inventory_changed` criterion, so any legitimate
acquisition counts. The item icon field is adapted to Horizon's current `id`
format. The original advancement background is covered by
`third_party/tconstruct/LICENSE`.

An isolated Linux resource fixture on Horizon `02c8607ee8ebd216f2f535c1eb7b62b572830df2`
and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a` confirmed the local
starting book and crafted Puny book produced the two saved advancement records.
A save/reload preserved both criteria and their original completion timestamps.
A LAN guest given the Puny book received its own two records; the host did not
receive the guest's Puny advancement. The guest's advancement screen showed the
two nodes. The test used supplied materials/items and does not establish a
survival acquisition route. The original background asset is included, but its
exact display parity has not been established. Packaging, final-main integration
and Windows are not yet verified; Puny's item and recipe require the Puny guide
integration.

These are two advancement paths, not the complete original progression. Other
source advancements, custom triggers, conditions and guide content remain
unfinished or unverified.
