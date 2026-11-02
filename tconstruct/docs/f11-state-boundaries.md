# F11 platform and channel state investigation

This records a candidate implementation boundary, not completed block support. Source references are TConstruct 1.20.1 `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`; the host observations below are from Horizon `9a6f7dd5a193d709f389adeb1f51858465b8f747`. Neither behavior has been executed here.

## Gold Platform and Scorched Basin

`shared/block/PlatformBlock.java` uses six directional booleans plus `waterlogged` and 64 directional outline/collision shapes. Its placement and each neighbour update compute the directions; `getBlockSupportShape` is a full block. `tconstruct:gold_platform` is an ingredient in the original Scorched Basin recipe.

A plausible existing state template is `minecraft:sculk_vein`: the host's `level/sculk.rs` uses six directional properties and `waterlogged`. This is a candidate encoding, not a working substitute. The template also uses `Class::Multiface`: `level/place.rs` only places it against an attachable face, `level/survive.rs` rejects absent or unsupported faces, and `level/shapes.rs` can replace it with air or remove a face. These differ from Platform. The existing `source-machine-prepare.rs` already rewrites selection/collision tables and could be extended to source Platform's 64 shapes; its current key is only block ID plus facing and must instead distinguish all six properties.

A complete implementation needs source-equivalent placement, survival and neighbour-shape updates on the authoritative world path, water tick handling, a full support shape, source assets/recipe, and the geometry table. Merely declaring the candidate template would create incorrect survival and collision behavior. Verify all six directions independently, no-neighbour placement, water, two platforms above/below and horizontal neighbour placement/removal, save/reload, local/LAN and the Scorched Basin recipe.

## Channels

`smeltery/block/ChannelBlock.java` has `down` and `powered` booleans and four horizontal three-valued `in/out/none` properties. `ChannelBlockEntity.java` maintains a fluid buffer, input direction and per-side capability/flow state. The official content declaration in `docs/modpacks.md` copies a vanilla template's states and behavior; it does not expose arbitrary new state properties. A vanilla redstone-wire state has directional three-valued fields but its value names, power field and redstone behavior differ, so it is not a drop-in channel.

A complete implementation needs a representable and persistent four-side direction state plus down/powered, source placement and neighbour/player/redstone changes, correctly sided fluid input/output, staged transfers and buffer persistence/synchronization, and directional source shapes/render. Verify each side and down, reversed/closed connections, power transitions, cycles, incompatible fluids, blocked output, chunk unload/save and LAN. Do not register a visual-only block as a functional channel.
