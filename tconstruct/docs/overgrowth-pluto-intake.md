# PR706: Overgrowth on the native carried-slot owner

Source candidate against integration `8360105dc27ee056df191db873853206b4dd3dc9`, official Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf` and Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`. Rust/native/plan/gameplay remain unrun. No PR closure or main adoption is justified by this source unit.

## What is actually connected in the postimage

Three private Game modules and one TAIL hook are appended. The native inventory callback supplies the current mutable Game and exact carried slot. Existing native per-player enumeration includes inventory, armor and offhand; this patch does not scan them again or invent an actor ID. The slot remains owned by that callback until its native cleanup/client synchronization. Remote clients, dead/absent players, disabled TConstruct, active-use slot, nonmodifiable or unsupported data do nothing.

The TC rule is still 5% per admitted effective integer level, one restoration multiplied by `1 + overworked`, capped by the existing derived capacity. TC retains definition, tag, trait and overslime policy. PlutoLib already has the needed operation: `horizon_client::pluto_nbt_storage::patch_compound`. This consumer uses it for exactly `tic_overslime`, preserving unknown native Tag/JavaString data and leaving the original snapshot untouched. No second NBT codec, registry, live-storage owner or generic event is added.

## Deliberate corrections to the selected old PR

[PR706](https://github.com/sofia-gros/horizon-mods/pull/706), fixed head `e2ea4c2f909406a684f4d33e826b9f97225ec7d2`, used a ChunkMap actor-phase HashMap, one whole-inventory TAIL pass, and `world.random.next_float()`. Those are not the original Java contracts. The postimage instead reads `player.health.tick_count % 20 == 0`, uses one native slot callback, and a TC-owned process-local `LegacyRandom` stream. A zero amount may be absent, as the original capacity bar removes it; wrongly typed amounts remain rejected.

[Original OvergrowthModule](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tools/modules/OvergrowthModule.java) uses holder tickCount, reference-identical active-use exclusion, and `Modifier.RANDOM`. [Modifier.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/Modifier.java) owns a static `java.util.Random`. Native LegacyRandom supplies the same 48-bit algorithm and 24-bit float draw, without advancing world generation/weather RNG. Startup initialization is nondeterministic; neither Java's startup seed nor interleaving with not-yet-migrated modifier draws is reproduced or claimed. This stream is not saved, just as that original static is not world save data.

The old PR's supported definition, integer levels, checked arithmetic and `capacity <= 32767` admission remain. The current ordinary overslime producer itself clamps to 32767. This does not claim all Java fractional effective-level/custom-definition or multiplier behavior is complete. Those inherited limits must not disappear from the remaining PR-coverage decision.

## Evidence and remaining gates

- Whole-manifest inverse removes only the appended three modules/one hook and recovers all base bytes, modules, hooks and states.
- Ten pure Python checks cover exact source hashes, inverse refusal, actual-slot routing, clock/source ownership, one-field Pluto delegation, pre-draw eligibility, and an independent Java-LCG integer reference vector. These are source checks, not Rust execution.
- Eleven actual Rust tests are prepared inside the three new modules: six policy, four native Tag/Pluto patch consumer, one actual LegacyRandom. They are unrun.
- Fresh complete plan must accept the exact native callback and bindings; all unchanged guards must pass. Same applied host must compile the real modules and execute those 11 exact tests. No mock Game is substituted.
- Native local/LAN, active hand, armor/offhand, slot transfer, tick freeze, same-mode repeated callbacks, death/rejoin and save/reload need actual behavior checks. A same-mode native callback is not a once-only scheduling proof by itself. No new phase or deduplication state is installed to hide that uncertainty.
- Existing higher-level slot/component synchronization is delegated to the official owner. Unknown NBT preservation here is a native Tag patch claim, not lossless arbitrary wire serialization.

Old PR files map as follows: the policy/world/tick files are superseded by these exact implementations; its phase-state manifest is replaced by the native slot hook; its docs/test requirements are represented here and by the prepared test inventory. The old condition-snapshot harness is not copied because the new consumer needs the actual native Tag/Pluto modules. Own-diff coverage and native gates remain required before closing PR706.
