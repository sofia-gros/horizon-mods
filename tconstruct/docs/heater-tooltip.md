# Heater fuel tooltip

The Heater tooltip uses the persisted Melter temperature, matching `GuiFuelModule.addTooltip` at TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. Positive temperature displays Solid fuel and that temperature; zero displays No fuel found. Queued fuel does not create a temperature before consumption. Exhausted fuel does not suppress a still-positive stored temperature.

This changes display only. Fuel consumption, temperature reset timing, tank tooltips, flame rendering, cursor bounds, and save/network formats are unchanged. Both local and remote screens continue reading existing persisted state. It does not guarantee that a stale remote snapshot is current.

The production Melter method is tested over remaining fuel 0/1/100 and temperature 0/1/800/1000, before and after real NBT serialization. Python checks the actual local/remote tooltip wiring and absence of queued-fuel inference. No live Game rendering, multiplayer session, or visual hover execution was performed.
