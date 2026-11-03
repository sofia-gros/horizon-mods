# Smeltery item intake

The Smeltery and Foundry check for dropped item entities on a twenty-tick cycle when a recognized fuel tank is present. They stage one item per empty melting slot and update ground entities only after controller storage succeeds. Living-entity damage and entity-melting recipes are separate and are not implemented by this path.

A supplied-item fixture on Linux with Horizon b53 and Mixins ddd checked representative Smeltery and Foundry intake, outside/no-tank rejection, item counts and components, save/reload, and host/guest synchronization. The current integrated head, failed controller writes and all boundary cases remain unverified. The initial apparent duplication report was retracted after a clean fixture preserved the original count.
