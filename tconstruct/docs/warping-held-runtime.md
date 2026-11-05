# Held Warping teleport runtime

Runtime slice of issue #379, stacked on Warping acquisition (#854). It follows
`SlingTeleportModule`, not the deprecated `WarpingModifier`, from
SlimeKnights/TinkersConstruct commit
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`.

## Supported path

- An intact stock melee/harvest tool with a saved Warping level of one can start
  the ordinary right-click item-use lifecycle. Current durability and charge
  tags, current material traits, the canonical modifier definition and native
  item/component conflicts are checked on the authoritative host.
- The allowed item set is pickaxe, sledge hammer, vein hammer, mattock,
  pickadze, excavator, hand axe, broad axe, kama, scythe, dagger, sword and
  cleaver. This is an item allowlist, not a promise that every material or
  modifier combination is supported. Unresolved definitions and competing
  general-interaction, sling or conditional-stat hooks fail closed. In
  particular, some built-in traits can exclude a configuration.
- Draw time is captured at start; the ordinary defaults give 30 ticks. Release
  uses the source partial-charge curve and projectile-damage/velocity defaults,
  Gaussian inaccuracy and the source per-axis quarter-block backward scan.
  The first non-suffocating in-border candidate is retained unless an
  intervening unbreakable block resets it. This deliberately is not a native
  raycast or full player-box collision test.
- Successful release centers X/Z in the selected block, preserves motion and
  rotation, clears fall distance, adds 0.2 exhaustion and a three-tick item
  cooldown, and charges one durability through the existing Overslime/wear
  chain with Warping as the cause. Broken tools remain in the inventory and
  equipment attributes are refreshed. Creative skips wear and native food
  exhaustion already exempts it.
- Success emits 32 portal particles and the source teleport sound; a valid
  positive-time release with no usable destination emits its lower-pitch
  failure sound. Sounds refer to existing vanilla assets.
- Native Stop cancellation and hand changes remain authoritative. The actor,
  slot and entire original stack must still match at release. The remote
  client only predicts the normal held-use animation, never the teleport.
  A one-shot, actor/position/rotation-bound LAN marker calls the existing
  motion-preserving teleport packet path before guest state is restored.
  State is transient and cleared with the world.

## Deliberate limits

This does not implement helmet/Skull-worn Warping, trait-only Warping, bows,
staves, shields, javelins, dual-action tools, or combinations requiring other
SlingForce/SlingAngle/SlingLaunch or conditional-stat hooks. It does not expose
Forge's cancellable SlingModifierTeleportEvent to unrelated mods. The native
block suffocation predicate is reused, including its existing shulker-state
limitations. The source's separate break animation callback is not added.

The scan never treats unloaded columns as air: it aborts without spending
wear, exhaustion or cooldown. Custom pack power above 32 or offsets over 1024
blocks are rejected to bound synchronous work. Native consumable, blocking
and cooldown components are excluded rather than run alongside Warping.
Client prediction uses only the safe saved-stack hint; unsupported live pack
or modifier combinations can therefore briefly animate before host rejection.
Changing the stack while charging fails closed instead of migrating state.

Four pure tests were added but not run. Validation is brief static review,
Rust formatting, TOML/JSON parsing and whitespace inspection only. Review
changed the module gate to a positive allowlist so implicit interactions and
unknown modules cannot bypass it. No tests,
build, planner, CI, version checks or merge were performed. Integrated-host,
dedicated-server, LAN, reload, breakage and visual behavior need the later
combined in-game test pass. No paused unpublished branch was incorporated.
