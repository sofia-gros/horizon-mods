# Smeltery and alloys

The Smeltery is in development. This work aims to connect a valid multiblock structure, liquid fuel, multiple stored fluids and source alloy recipes with the existing faucet and casting setup.

The first acceptance scenario will melt two materials, combine them using a source alloy recipe, pour the alloy and collect a cast item. It also covers structure validation, saved state and LAN use. This is a plan, not a claim that those operations work yet.

The planned fuel supply uses a lava bucket on a seared fuel tank or gauge in the structure. The original tank accepts fluid containers directly; the current implementation attempts the same lava-bucket interaction. The two container slots in the Smeltery screen are not implemented yet and do not accept items. Other fluid containers are also not supported yet.

The Foundry and specialized recipes are separate work.
