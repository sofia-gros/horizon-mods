# Contact timing and native effect attachment: official feasibility

This is a bounded read-only study of official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. It adds no installed hook, state,
consumer activation, alternate actor owner or native source rewrite. The prior
immersion/curative building blocks have separate complete-package evidence in
[immersion-curatives.md](immersion-curatives.md).

## What the source behavior requires

TConstruct's pinned `MobEffectLiquidBlock.entityInside` first tests the entity's
cached fluid-type height for a positive value. For a living entity it creates
an effect, assigns an explicitly empty curative-item list, then calls addEffect.
The distinct `BurningLiquidBlock` checks fire immunity and cached immersion,
sets fire, hurts with lava and only then conditionally plays the burn sound.
PlutoLib must not choose effect IDs, burning values, sound/random policy, or an
owner absent from those callers. A later world query is not the original cached
value merely because its numeric height happens to match.

Forge's effect update has distinct ordinary-update and details-copy operations.
The curative list belongs to the instance, including saved hidden instances.
A global rule for an effect ID would incorrectly affect a potion-created
instance of the same effect. Restoring an effect after a blanket clear is also
not the original cure operation's removal/callback order.

## Official synchronous route and its limits

The relevant player route is:

1. `MotionWorld::fluid_base_tick` calls `update_fluids`. It temporarily takes the
   player's `EntityFluidInteraction`, calls the native update and restores it.
   Its interaction box applies native body deflation and the current ridden-boat
   adjustment. The native tracker uses closed water/lava `FluidKind` values;
   mod ordinary blocks do not become native fluids automatically.
2. Local `local_tick`, `simulation_tick` and `player_movement_tick` call this
   base phase before travel. `dry_tick` travels and then applies block effects.
3. `check_inside_blocks` sweeps the actual movement segments, tests native
   collision exceptions and deduplicates positions. Its synchronous Inside
   notification happens before the built-in per-block effects in that loop.
4. On the guest path, `as_guest_with_host` swaps the actual player's context and
   enters the correct contact dimension. `guest_server_tick` in lan_actions.rs
   runs fluid_base_tick, reconstructs movement history and runs server_block_damage
   before collecting events. Wither-rose events are applied later in Game, then
   the normal effect tick runs. This is evidence of native behavior, not permission
   to delay a custom fluid callback into the same later queue.

`block_contact::HOOK` receives only dimension, the copied contact facts and a
mutable fall-damage scalar. `Game::native_block_entity_inside` is also a static
function, with no Game, actor-effect collection, immersion snapshot or guest
capability. `MotionWorld` has the live PlayerMotion but no PlayerEffects; the
latter is owned by Game's PotionState. A numeric entity ID or dimension is not
permission to reborrow either owner during simulation. Re-entering Game through
thread-local pointers, a raw address, or a later saved roster is not used.

A read-only synchronous observer at a carefully selected MotionWorld site may
be possible. It would establish facts in that live borrow only; it would not by
itself supply the earlier fluid snapshot or effect mutation authority. Its
interior selector/borrow legality and exact ordering still require a separate
complete-plan and applied-type proof. No such hook is installed by this study.

The entity domains also differ. Mob effect storage is a vector of
`(effect id, ticks, amplifier)`, not `EffectInstance`. Its `hidden_effects` field
means hidden particles, not a suspended weaker-effect chain. Items and projectiles
also emit contacts but are not LivingEntity effect targets. A player-only adapter
must not be advertised as complete living-entity parity.

## Native instance attachment is deliberately unsupported

Official `EffectInstance` is a public, non-non_exhaustive struct with only public
fields. Mixins `state::plan` explicitly rejects that layout:

> state requires a private field or non_exhaustive struct; external construction would break

This rejection happens before constructor coverage or inserted-field typing.
The official documentation describes the same contract. This is not merely a
hypothesis about a later Rust error. The checked-in negative probe requests one
owned `Option<Vec<String>>` field and one constructor. It accepts only the exact
E_STATE diagnostic and no output plan. The probe is not a production package.
Execution status is recorded in [the evidence record](contact-effect-attachment-proof.json).

The rejected condition protects real callers: beacon and conduit code in the
higher gameplay crate use struct-update literals against the render crate's
EffectInstance. Making a field private or adding non_exhaustive without changing
those callers would break their legal construction. Making it public would
change the API contract rather than honor the supported state operation.
No attribute patch, planner bypass or copied native struct is used.

Even an approved owned metadata field would be only the first prerequisite:

- Derived Clone must copy metadata for each hidden instance independently.
- Ordinary `update` must retain the active policy, while hidden construction
  copies the correct incoming/previous instance. `downgrade` currently copies
  native fields individually and would need an explicit policy transfer.
- `saved` and `from_saved` currently encode five fields, without curative data
  or hidden chains. A durable contract must specify old-save defaults and
  lossless hidden-instance persistence, not silently drop the new policy.
- Native network reception replaces an effect with a fresh instance. Whether
  metadata remains authoritative only on the server or is synchronized needs
  an explicit protocol contract; network packets cannot infer it.
- `ConsumeEffect::ClearAllEffects` directly clears the collection and applies
  status. Targeted removal, commands, death and expiry are different operations;
  per-item cure logic must not hijack all clear paths.
- The existing mob tuple representation requires its own coherent owner/lifetime
  work; extending only the player's struct would leave that domain unsupported.

## Alternatives and recommendation

1. **Keep the current bounded API.** Explicit bounds/surfaces and immutable
   Curatives values are useful to callers that already own their contexts. They
   must remain labeled building blocks without native contact/cure activation.
2. **Obtain an official extensibility contract.** The smallest complete native
   path needs a supported owned-instance metadata boundary (and compatible
   external construction), explicit ordinary-update/details-copy/hidden/save
   semantics, plus a contact-side capability that retains the native actor and
   earlier immersion context. PlutoLib can adapt that contract internally while
   TConstruct supplies only its original content policy. This study does not
   modify or publish changes to the official repositories.
3. **A separately owned mod effect runtime is not a drop-in fix.** It could own
   tagged values and hidden chains safely, but synchronizing attributes, ticking,
   saves, native commands and network state would replace substantial gameplay
   behavior. It is outside this bounded migration and is not selected silently.

Do not use an effect-ID map, pointer map, global current-player selection,
metadata encoded into native IDs/durations, or clear-and-readd compensation.
None establishes the required instance lifetime and synchronous authority.

## Pinned source links

- [Player fluid phase](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/motion/fluids.rs#L287-L323)
- [Player sweep and contact order](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/motion.rs#L1711-L1894)
- [Guest context and contacts](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/lan_actions.rs#L340-L355)
- [Native contact capability](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/block_contact.rs#L10-L87)
- [Effect storage and transitions](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client-render/src/effects.rs#L8-L104)
- [Mob effect representation](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/mobs.rs#L1192-L1198)
- [Official state rejection](https://github.com/horizon-rust/horizon-mixins/blob/e8cd0aa38cffef49cb2e088ef716baa107148c02/crates/horizon-mixins-core/src/state.rs#L72-L88)
- [TConstruct liquid contact](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/block/MobEffectLiquidBlock.java#L28-L34)
- [Forge per-instance policy](https://github.com/MinecraftForge/MinecraftForge/blob/d13dd327bdc75c010f3358992e80e7e92f51e030/patches/minecraft/net/minecraft/world/effect/MobEffectInstance.java.patch)

## Observed negative capability proof

The frozen `55a9e8ed` probe was executed against the pinned official CLI and
source export. It returned exactly `E_STATE` with the public-layout rejection
above, produced no plan, and left all 5,007 verified official export entries
unchanged. Elapsed time was 7.88 seconds; the continuous disk guard did not stop
the run (recorded minimum 10,995,793,920 bytes). No apply, compilation, native
effect insertion or gameplay test was performed.

The first attempt reached only `E_METADATA` because cargo was absent from PATH;
its failure is preserved separately. The same frozen probe then ran with the
approved environment. `plan` invokes Cargo metadata internally, so a working
Cargo PATH is required even though no build/test is run. The runner accepted
only the exact expected state rejection, not that earlier environment failure.

This establishes a real unsupported planner boundary. It does not establish
that every possible future host extension is impossible. The recommended next
step is an official owned-state/contact capability design, not a local bypass
or an assumed player/mob equivalence. The existing generic building blocks
remain installed separately; this negative probe is never shipped.
