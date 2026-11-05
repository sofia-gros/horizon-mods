# Bounded Worldbound dropped-item runtime

Related to high-priority #379. Source Worldbound sets the volatile `indestructible` flag, which makes tools and armor create an `IndestructibleItemEntity`. This change supplies the lifetime and native environment-damage parts of that behavior, not every custom-entity feature.

## Implemented

- Validate the canonical Worldbound definition, current assembled-tool traits/upgrades, the live modifiable tag, one-item count and non-opaque stack. Broken tools/armor still retain their volatile flag. No derived marker is added to the stack or inventory.
- On native expiry, repeatedly request the native maximum 32766-tick extension. This avoids the native unlimited-age sentinel, which stops item spin, while retaining movement, pickup delay, thrower, target, item components and native save fields. It does not implement a separate persisted custom entity type.
- Before the normal dropped-item tick, resolve a world-local set of protected entity IDs. Their environment step skips fire, lava, continuing burn and cactus damage before it happens, and clears burning. It does not heal or revive the item. Clear that temporary set after the enclosing item tick; never serialize it.
- Void removal, pickup, native merging eligibility and physics remain native. Remote clients do not infer protection from local resource data. Source resources unavailable or overridden outside the supported shape fail closed.

## Deliberate limits

The explosion extension in `worldbound-explosion.md` protects that separate native path. Lightning destruction remains unprotected. Other damage paths, source custom-entity identity and exact lifespan/age/rendering behavior are not claimed. Native age periodically resets for the extension, unlike the source's maximum lifespan field. Source custom-entity behavior survives independently after spawning, while this adapter revalidates current Worldbound data, so changed datapacks can remove its protection.

Worldbound acquisition, rare-name display, Soulbound death retention, custom volatile-flag producers, malformed/opaque stacks, non-stock item implementations, and generic invulnerability-bypass tags remain separate. Existing Netherite behavior is unchanged. A combined Netherite/Worldbound stack still passes through the existing Netherite return hook.

## Review

Brief source/static review and Rust formatting only. Two pure tests were added for canonical-definition gating and expiry arithmetic, but not run. No tests, build, planner, CI inspection, version checks, or merge were performed. Final validation should cover expiry/save/reopen, burning/lava/cactus, void loss, broken tools, pickup integrity, reload and LAN behavior.

Source: TinkersConstruct commit `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, `worldbound.json`, `IndestructibleItemEntity`, and `ModifiableItem.createEntity`.
