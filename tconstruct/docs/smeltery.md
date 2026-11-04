# Smeltery and alloys

Build a small Smeltery to melt materials and cast an alloy. In Creative, use seared bricks for the floor and walls, a Smeltery Controller, a Seared Fuel Tank and a Seared Drain. Leave the space inside empty and the top open. Connect a Seared Faucet to the drain by clicking the drain face, and place a Casting Table beneath the faucet. Faucet orientation follows the clicked face rather than the player's direction; clicking the top or bottom face sets it to face down. The normal-click placement path still needs in-game verification.

Fill the fuel tank by using a lava bucket on it. Open the controller and add a copper ingot to the input at the left. Once it melts, add an amethyst shard. The molten materials combine into amethyst bronze. Place an ingot cast on the casting table, use the faucet, and collect the cooled ingot.

![Molten amethyst bronze in the Smeltery](smeltery-alloy.png)

The left panel holds the melting input. The fluid display shows the alloy in the controller.

This copper and amethyst sequence was tested on Linux in single-player and LAN. The single-player test also checked saving and reloading, and breaking and repairing a wall while an input was present. The two container slots in the Smeltery screen do not accept items yet; use the fuel tank directly. Other fluid containers are not supported yet.

Larger structures, other alloys, the Foundry and specialized recipes need further testing or implementation. Machine shapes and visuals are not yet a complete match for the original. Windows has not been tested.

Seared and Scorched Chutes connect adjacent hoppers to the formed controller's melting slots. A hopper facing a Chute inserts one item into the first empty melting slot; a hopper below a Chute can extract one item into an eligible hopper slot. The move is staged on both sides; after the native inventory callback it rechecks the structure and both snapshots before committing, and restores the hopper if the controller write fails. Opaque network component patches are conservatively refused. Representative b53 single-player hopper transfer and save/unload cases were tested; the updated callback guard, LAN and current host remain unverified.

Brewing stands can also turn a glass bottle plus Earth, Sky, Ender or Ichor Congealed Slime into the corresponding slime bottle, or a glass bottle plus a Magma Block into a Magma Bottle, when the required items are registered and no existing recipe overlaps. World reload, integrated host, multiplayer and gameplay have not been verified for these paths.

### Built-in tank to casting

A Faucet can draw from a registered Seared or Scorched fuel/ingot tank and pour into a casting table or basin when the source, faucet, and casting save data can be safely decoded. A changed or missing recipe, unknown source fields, unreadable target, or unloaded chunk causes this new route to wait rather than rewrite uncertain data. Source, target and faucet edits are staged together in memory. Existing behavior when the target is removed may discard the reserved packet, as in the source; this does not guarantee atomic disk or network persistence. Generic modded fluid handlers, live game, save/reload and LAN remain unverified. The route fixtures use real state types with a mocked world and recipe selector; they do not establish an integrated host build.

### Drain to built-in tanks

A Faucet can move untagged fluid from a formed Smeltery or Foundry Drain into built-in fuel and ingot tanks. The source structure and Drain ownership are revalidated; a stale controller, unloaded chunk, Duct, tagged fluid, unknown tank representation, or unreadable target waits without draining. The controller's original compound is retained and only its canonical tank is replaced. Initial and subsequent packets begin by transferring up to 10 mB, with controller, destination and Faucet changes staged together in memory. When the destination changes during a reserved packet, the existing Drain/Duct tick paths also preserve the raw controller compound and change only its canonical tank; unknown tank formats wait. Existing target destruction can discard a reserved packet. The route does not cover Duct filters or third-party handlers and has not been verified in the full host, a live world, save/reload or LAN; the state fixture uses a mocked world and structure detector.

### Built-in tank to Casting Tank

An untagged fluid from a built-in fuel or ingot tank can travel through a Faucet into a Casting Tank. The route stages source, output and Faucet data together, processes a supported container after fluid arrives, and updates the live slots only after the staged write succeeds. It rejects noncanonical saved tank data or live-slot mismatches; old Casting Tank and Cannon tick paths now also wait rather than overwrite noncanonical data when the source or target changes. Tagged fluid, generic handlers, full host build, live world, save/reload and LAN remain unverified. The route test uses real state/NBT types but a mocked world and container transfer, so it does not establish actual bucket integration or disk/network atomicity.
