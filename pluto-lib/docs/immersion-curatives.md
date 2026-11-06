# Synchronous immersion and per-instance curative building blocks

This group adds reusable numeric/policy contracts, not live contact effects or
native instance attachment. TConstruct effect IDs, timings, burn rules and owner
selection remain outside PlutoLib. Targets remain official Horizon `624c4f62`
and Mixins `e8cd0aa3`; no host repository is changed.

## Explicit synchronous height query

`pluto_api::fluid::contact::immersion::query_with` accepts already-adjusted bounds,
the existing contact Limits, and a synchronous read-only surface provider. It
reuses the existing coordinate/budget preflight before any sample, iterates X/Y/Z,
and returns maximum `surface_y - min_y` per fluid identity. It retains tangent
zero heights, reports unloaded cells and never treats an incomplete result as
proof of dryness. It does not sum intersected volumes.

The provider supplies the absolute surface coordinate and its precision. The
query does not invent body deflation, boat adjustment, filter, actor identity,
tick cache or contact timing. `View::query_heights` delegates through the existing
loaded-before-read decoder and current stepped-cell geometry. That geometry
continues to use its existing f64 heights; it is not presented as Forge's
above-fluid or float32 surface rule. No registry or renderer ownership changes.

Forge's inspected Entity patch uses a body deflated by 0.001, evaluates the
float32 world-surface addition, accumulates maximum height per fluid type and
stores it during the fluid update. A later TConstruct contact callback reads
that cached value. Recomputing a body from a later guest roster or queuing the
callback would not prove equivalence. The new API is synchronous in its supplied
context, not a claim that it has been installed at that original simulation site.

## Immutable policy owned by one effect instance

`pluto_api::effects::Curatives` distinguishes an unresolved default from an
explicit list, including empty. The caller supplies default item identities;
there is no implicit milk rule. Resolution happens once, lists keep their order
and duplicates, and matching uses exact canonical item identity, not tags/counts.
Cloning copies the owned list independently. Limits are 256 entries and 256 bytes
per canonical namespaced ID; these are API admission limits, not source game rules.

`Transfer::OrdinaryUpdate` retains the current instance's policy.
`Transfer::DetailsCopy` copies the incoming policy, including its unresolved state.
This reflects the inspected Forge patch's copy-details region rather than
assigning incoming cure metadata whenever duration/amplifier changes. The caller
must identify the actual operation and own storage/hidden-chain lifetime. This
value does not automatically follow native effects and does not remove any.

## Native attachment remains blocked

Official render `EffectInstance` has public fields and cross-crate struct-update
literals in `game/beacon.rs` and `game/block_entity_anims.rs`. Current Mixins state
fields are private. Static Rust privacy analysis therefore identifies a barrier
to adding an instance field without changing those callers. No actual applied
state attempt failed in this group; it has not been attempted. A pointer/ID side
table would not reliably follow moves, clones and hidden instances and is not used.

Native cure invocation, hidden restoration, full save/reload and server/client
transport still require an explicit safe owned adapter. `PlayerEffects::clear`,
commands, death and expiry remain unchanged. No TConstruct caller is activated.

## Evidence and validation scope

TConstruct source `f4e77d43` sets an empty curative list in MobEffectLiquidBlock.
Forge sources were read at immutable `d13dd327bdc75c010f3358992e80e7e92f51e030`
on its 1.20.1 line. The effect patch's last file change on that line is
`44c689f1712fcfb7ccb13262d1c02f0a880055be` (2023-02-24). This is pinned source
lineage evidence, not a verified Forge 47.4.0 binary artifact.

- Effect patch blob: `39dd6908d36deb446bbff832779dbcb31c63dce7`.
- Cure interface blob: `7f587e5200b17337d4f9a874b0158904fcda2aa1`.
- Entity immersion patch blob: `17e5587035c4b550c027f7b7288e3bbe2c15e647`.
- LivingEntity curing patch blob: `0c3ee08fe7a59da02dbb168cea6012beda055f1a`.

Sources: [Forge effect patch](https://github.com/MinecraftForge/MinecraftForge/blob/d13dd327bdc75c010f3358992e80e7e92f51e030/patches/minecraft/net/minecraft/world/effect/MobEffectInstance.java.patch),
[cure item identity](https://github.com/MinecraftForge/MinecraftForge/blob/d13dd327bdc75c010f3358992e80e7e92f51e030/src/main/java/net/minecraftforge/common/extensions/IForgeMobEffectInstance.java),
[fluid-type height](https://github.com/MinecraftForge/MinecraftForge/blob/d13dd327bdc75c010f3358992e80e7e92f51e030/patches/minecraft/net/minecraft/world/entity/Entity.java.patch).

At source `697a851a`, the actual API crate passed 36/36 tests, including all
twelve new kernel/policy tests. The minimal official-loader fixture passed 9/9,
including its new actual View height test. Both had zero ignored tests; the
strict fixture source/dependency checks passed before and after. The continuous
disk guard did not stop either run; final free space was 11,052,900,352 bytes.
The complete combined package gate remains separate. Evidence is recorded in
[the proof record](immersion-curatives-proof.json). Neither fixture proves live
contact cadence, native effect attachment, gameplay assets or curing in Game.
