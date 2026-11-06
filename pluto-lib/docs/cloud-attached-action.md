# Attached-player cloud migration candidate (not adopted)

**Not adopted for ordinary right-click repair:** the existing packet action holds its roster outside `Game.lan`, so this attached-only boundary does not establish a supported ordinary action path. This checkpoint preserves a bounded unfinished candidate; integration must not activate it as the requested feature.

This is a source candidate on MOD base `e128d61478971e247b1e3e2e09516cf99652f9fc`, for official Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf` and Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`. It has not been planned, applied, Rust-compiled, or run in Game. The existing 19 other cloud-guard composition conflicts remain rejected. Do not interpret this checkpoint as a playable cloud feature or as complete latest-host compatibility.

## Concrete defect and bounded change

The original `source-fluid-cloud.rs` directly constructed native `Breath`/`PotionCloud` and allocated an ID. Its initializer omitted current required `Breath.victims` and `PotionCloud.settings/owner/guest`. Adding fields or inventing an owner would not establish the action's attribution.

The replacement keeps TConstruct's policy in that consumer and delegates validation and publication to the existing Pluto boundary. The consumer module moves from `game/potions.rs` to `game.rs`, since it no longer accesses private native potion structures. This removes that one private-location conflict without adding an allowlist. Manifest changes otherwise reverse byte-for-byte to the integration base; no injection is added.

The new Pluto private `AttachedPlayerFrame` has non-public fields. Its capture requires the actual Ready authoritative native lifecycle, native LevelRef and EntityRef lookup, current motion, and an attached, placed, loaded, non-dead roster entry with matching dimension, connection, entity ID and UUID. It does not select the current player as a fallback, infer ownership from a numeric ID, accept Unowned/None attribution, swap a guest, create a queue, or keep a Game borrow alive. It records an observation, not a new permission or proof of held-item identity. Consumers still must be handling the matching action in the same invocation.

Capture and publication recheck the current frame. The original `resolve_owner`, attribution equality, range, loaded center, capacity reservation and take-ID/assign-ID/push sequence are byte-identical. There is no rollback after publication. The broader pre-existing `Owner::Unowned` API is not removed, but this consumer never uses it.

## Known unsupported ordinary paths

Official `command_guest` reads only `self.lan.as_ref()?.hub`. A detached `as_guest` body with its roster held in a separate `&mut LanHost` cannot be resolved by this API and fails closed.

This is not merely an unusual case: native `UseItemOn` enters `as_guest(host, conn, ...)` and calls `use_item`; `game.rs` then reaches `use_tool` and `use_block_transformer`, where the existing TConstruct cloud hook calls this helper. That route does not put the external host back into `self.lan`. An attached command frame exists in `run_guest_command`, but this source review has not established a normal held-item cloud action reaching that frame. Consequently **ordinary player right-click cloud functionality remains unproved and the detached route is intentionally unsupported**. A genuine owner-supplied roster/action boundary is needed to support it; this candidate does not fabricate one.

The admission also rejects remote/client-only worlds, non-Ready lifecycle, absent/mismatched motion or roster, wrong dimension, stale native handles and a changed frame. Observer equality is not a general reentrancy, disconnect/reconnect ABA, selected-slot, or continuous actor-lifetime guarantee. Complete installed composition and actual Game tests remain mandatory.

## Content and consumption policy

The consumer keeps radius 1, radius-on-use -0.5, 600-tick lifetime, shrink -1/600, wait 10, reapplication 20, duration-on-use 0, 4x stored duration and 0.25 native duration scale, visible/non-ambient flags, the area-effect-cloud translation key, original effect order and duplicates, and native weighted RGB for the admitted finite domain. These choices are not shared-library defaults. The caller's existing effect selection, >10-duration filtering, scaling, placement choices and non-cloud transformations remain unchanged.

The shared descriptor has a 256-effect budget and amplifier range 0–255. Empty, unknown, overflowed or unrepresentable requests now fail explicitly before native publication rather than constructing malformed native objects. This is a representability restriction, not an assertion that every old input is supported. Original source TConstruct `MobEffectCloudFluidEffect.makeCloud` uses its explicit `context.getEntity()` as owner; this candidate preserves the need for action-derived attribution rather than guessing that entity.

Four exact edits in the existing block-action caller are recorded in `cloud-attached-action-inverse.json`:

1. Capture before cooldown/world effects. A pure-cloud unsupported context returns an explicit error before charging. For mixed glow+cloud, retain the original glow-only attempt and report cloud unavailability.
2. Track successful cloud publication separately.
3. Mark `cloud_spawned` only after a successful shared insertion. Existing staged fluid remainders are prepared before side effects, and original max-successful-effect accounting remains.
4. After any publication, recheck the actor and held stack before the existing final fluid/wear commit. A changed actor or held item causes an explicit partial-error result and withholds the item commit. Already-published clouds or other effects are not rolled back.

Mixed-effect callbacks, later wear callbacks and arbitrary other mods are not proved safe by this source test. The untouched existing error accounting can still report partial world work. No claim of atomic all-target action/consumption is made.

## Evidence and next gates

- 12 new source/inverse checks and 6 updated existing source checks pass. They verify exact edits and ordering; they are not execution of Rust or Game.
- Five new Rust tests are prepared but unrun: two actual native-handle value checks and three consumer descriptor/color/validation checks. These tests alone would not prove Game capture or action reachability.
- Original insertion and owner-resolution bodies reverse exactly. The original guard algorithm and exclusivity rules are unchanged. Metadata includes the reviewed 41f 17 inputs plus exact `lan/guest.rs` and `motion.rs` type inputs. Actual plan input inclusion and composition of these additional owners are unverified.
- The older published 979 metadata-only candidate asserted unchanged runtime. It is a separate historical cutoff, not an extra test layer to append blindly to this runtime delta.
- Required next gates: complete actual plan and all applicable guards; actual module placement, visibility and host type check; the five prepared tests; real Game rejection/success fixtures with owner-supplied action evidence; callback/held-item continuation and partial effects; actual packet/ordinary input behavior. Until those gates, actual_plan and host_type_check remain null.

No official checkout, official repository, existing proof cutoff, or forbidden fork is modified by this candidate.
