# Melting and casting

The Melter and casting setup is under development. The first playable setup is intended to melt supported items using a heater, move liquid through a faucet, and cast an item on a casting table or in a basin. It is not yet available to play until that sequence has been checked in the game.

The implementation uses the original recipes, the original Melter ore-rate defaults and the source fluid textures where available. Fluids without a retained source texture use a generic visible texture; animated fluid frames and the casting cooling fade are not yet implemented.

Fluid-container interactions, redstone control and specialized recipe types remain separate work. The larger Smeltery, Foundry and alloying are also separate. Machine collision shapes still need comparison with the source before they can be described as accurate.
