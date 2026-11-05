# Strong Bones milk-bucket effect

Related to high-priority #379. Stacks on #772 to extend its single milk-consume wrapper rather than introduce a competing wrapper.

## Implemented

- Finishing a milk bucket while wearing a supported stock helmet with Strong Bones grants ordinary Resistance I for `600 + 600 * level` ticks. Native consumption, bucket return, effect add/update and hidden-effect behavior remain in charge. This runs after milk curing and also allows creative players, matching the source finish event.
- Resolve current material traits/upgrades and the live modifiable tag. Strong Bones is static Java code; a datapack replacement definition is unsupported and fails closed. Broken helmets are allowed by the source modifier lookup.
- Track the source helmet item type on the live Resistance instance. That instance survives subsequent milk clearing, alongside the existing supported Insatiable/Momentum effects.
- Cure the owned active instance when the helmet is removed, loses Strong Bones, or changes item type. Swapping to the same item type with positive Strong Bones keeps it. Ordinary Resistance with no matching cure marker is not removed.
- Transfer cure ownership when the native active effect is actually replaced, keep it with hidden copies, and restore it on downgrade. Temporary update bookkeeping is cleared. No tool or armor inventory metadata is modified.

## Important limits

The custom cure-item metadata is runtime-only in this PR. Native saves still retain Resistance, but not this marker: after reopening, the remaining effect behaves as ordinary Resistance and cannot be recognized for helmet-removal curing. Custom metadata transport, save/reload parity, and exhaustive LAN/guest transition behavior remain for integration work.

Only milk-bucket curing is covered. Calcified, milk fluid spilling/slurping, generic non-milk curatives, custom helmets, generic source event cancellation, and arbitrary replacement definitions are separate. The old native host has no per-instance curative-item list, so this is a narrow helmet-owned marker rather than a complete Forge curative system.

The separate host registered-fluid curative work adds `no_curatives` and `PlayerEffects.cure()`. Final integration must combine that policy with the existing #772 wrapper's whitelist and this marker, preserving generic `cure()` semantics and leaving direct command/death `clear()` unrestricted. This branch neither changes that host work nor claims it is already integrated.

## Review

Brief source/static review and Rust formatting only. Two pure tests were added for duration arithmetic and active-instance replacement decisions, but not run. No tests, builds, planner, CI inspection, version checks or merge were performed. The final pass must validate the generated EffectInstance fields, hidden upgrades/downgrades, repeated milk, helmet swaps, death/commands and the documented persistence gap.

Source: TinkersConstruct commit `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, `StrongBonesModifier`, `CureOnRemovalModule.HELMET`, and `ModifierUtil.getModifierLevel`.
