# Mining context migration: independent source review

Status: read-only source review, not a plan, apply, compile, differential test or gameplay result. Storage owns implementation. No ordinary manifest, old proof or fork was changed or read. Template/default CI remains frozen at be7b6b98 and unstarted.

## Fixed inputs

- Mod main: 08afa992d5091e03360f3473ea93fd17aa793987.
- Old official comparison: Horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0.
- Assigned latest official: Horizon f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd; Mixins 5a64af938ec86b00d53d16a408aabc0038fb3e57.
- Existing common library: 035273c4805d75111e528c8c26607fa3a8d1d5bf, ordinary archive SHA256 4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d. This is still an old-target proof, not latest validation.
- Exact paths and source hashes are in evidence.json. Git blob identifiers and SHA256 are labeled separately.

## Result

The old host speed-local hook cannot simply be renamed or retargeted. Latest tick_mining delegates to destroy_progress(hit.pos), while destroy_progress now invokes the official native_break_speed hook after native speed/environment calculation and before hardness/harvest division. That formal, synchronous hook is a substantially narrower migration boundary than re-creating host/client lexical captures.

Keep the separate Items::mining_speed RETURN policy: it is the base Tool/broken-state stage, not this actor/environment stage. Replace the two host/client context hooks as one coherent change to avoid double application on the new shared route. A private Pluto Game-side adapter can use formal pos/state/current speed and the same actor borrow; TConstruct retains its material IDs, modifier interpretation, resource snapshots and actor-specific wire authorization. No new global/TLS/cache or snapshot owner is needed for this proposal.

## Old behavior that must not be silently lost

The existing source-mining-context-shared.rs apply function is computation-only but depends on private Game and consumer state:

1. Armor Haste reads armor[1] and applies even when the held item is not TConstruct, lacks custom_data, or has an invalid Tool. The remote path accepts only the matching authoritative Haste frame for that stack; local/server uses swapped acting-player equipment, resolved harvest tags, modifier resource snapshot and material traits. Missing remote evidence contributes zero trait levels, not a local guess.
2. TConstruct held-stack policy requires actual custom_data and Tool, with evaluated-speed tri-state behavior: malformed Tool preserves incoming speed; missing/not-effective Tool is different. Tier exclusions, broken flag/durability and material modifier order remain consumer policy.
3. Broad membership distinguishes unavailable from false. Remote membership comes from its wire state; local membership comes from resolved tags. Unknown suppresses only the relevant Overwield addition.
4. Source flat-bonus multiplier is intentionally distinct from native base speed: max(Haste, Conduit), source fatigue table, current eye-water/Aqua Affinity and on-ground. The existing tests explicitly retain a host/source fatigue difference and do not multiply flat additions by an unrelated block-break-speed attribute.
5. Hydraulic uses previous eye-water (`was_eye_in_water`), not current eye-in-fluid. On-ground defaults true when no player exists; current water defaults false; previous-water remains unavailable. Airborne and Armor Haste are applied after conditional additions in that order.
6. The old host helper obtains y from game.mining, whereas the client helper takes pos. The new native formal pos is the actual block being evaluated, including guest STOP/delayed calculations when game.mining may not be the queried block. Use explicit pos/max_y, not a fresh global current-target cache.

## Official call order and authority

- Latest game.rs 7572-7644: tick_mining preserves restriction/creative/delay/unbreakable handling, calls destroy_progress(hit.pos) once, then updates swing/progress/ticks and performs the existing server decision.
- Latest game/hand.rs 389-421: Items mining speed; correct-for-drops; modify_destroy_speed; block-break attribute; eye-position water/submerged attribute; off-ground divide; native_break_speed; hardness and 30/100 division.
- Latest game/native_hooks.rs 2621-2644 documents local, acting LAN/dedicated guest and remote prediction, compute-only behavior, current speed modifications and original baseline. Its body dispatches ordered trait mods.
- Latest game/lan_actions.rs also evaluates destroy_progress for guest start at 2069, stop at 5126 and delayed completion at 5158; game/hand.rs 831-832 forwards client click progress. Therefore do not promise only one callback per user tick or make the callback spend items/update persistent state.
- An injected HEAD at native_break_speed follows the official documented mixin-before-trait order. It should mutate only the current mutable speed; the original formal is unchanged. RETURN would put TConstruct after trait mods and is a different composition contract. The old version had no such trait event, so this ordering must be selected and tested rather than claimed as legacy multi-mod equivalence.
- Official docs say speed zero does not break instant blocks. The actual latest expression still performs speed/hardness: for zero hardness and zero speed, the arithmetic result is NaN, not numeric zero. tick_mining's subsequent >=1 condition is false, but guest_delayed_destroy uses `Some(p) if p < 1.0 => {}` and otherwise its completion branch, so NaN reaches that different branch. Do not generalize the tick_mining result to all callers or invent a clamp in the adapter. Positive, negative and non-finite inputs require caller-specific source-aligned tests. tick_mining filters hardness<0, while direct destroy_progress explicitly checks hardness==-1; those are not interchangeable checks.

## Why old LOAD ordinal is unsafe

Latest Mixins scopes.rs 279-286, 397-425 and 478-490 shows that compound-assignment left operands suppress read collection; references are counted as LOAD paths but not supported rewrite values. variable_sites selects all matching accesses, without filtering unsupported ones.

The old hand destroy_progress has two speed LOAD expressions: modify_destroy_speed(speed), then final division. Latest hand has four: that first argument; native_break_speed's original speed argument; its &mut speed borrow (counted but unsupported); then final division. Consequently require=2 is no longer valid, and ordinal=1 now denotes the original argument rather than the final division. Reusing it would modify the original baseline instead of safely changing the event's mutable current speed. This is source-derived enumeration, not an executed planner result.

## Public API sufficiency

Existing Pluto pluto_tool_rules provides ordered Tool parsing/evaluation and Items mining-speed/drop RETURN hooks. It has no acting-player environmental context or consumer wire authorization service. Keep that layer separate.

The official trait hook is useful, but public Host alone is not a proven equivalent interface for the old policy. native_held returns only (item name, count, plain flag); NativePlayer holds position/orientation/dimension. Other equipment/entity helpers exist, but the catalog has no helper for the consumer's Haste/Broad world states or previous eye-water source fact. Moving the unchanged policy into a standalone trait crate would cross a real visibility/authority gap. A synchronous Game-side private adapter can preserve those borrows without inventing ownership. No claim is made that every conceivable combination of public helpers was executed or that a trait-only migration is impossible in principle.

The new break_speed entry is in tools/native-hook-fragments/break_speed.json, not yet the numbered native-hooks.json list at this immutable revision. Use the verified actual function target/fragment contract when designing the candidate; do not invent a numbered hook_version. Generated mod API construction reads the signature and fragment. The callback is a listen contract with mutable current speed, not a cancellation or block-breaking grant.

## Minimal proposed migration

1. Pluto private adapter at the official native_break_speed formal boundary acquires the current state facts and held item, plus pos/max_y. It does not alter original, native division, restrictions, actor selection, drop correctness or consumption.
2. The consumer callback calls the existing shared policy exactly once with those borrows and current speed; policy-owned world snapshot/frame reads stay in TConstruct. Prefer unchanged shared apply bytes for the first compatibility proof.
3. Remove both legacy lexical host/client context injections in that same candidate. Keep the existing Items RETURN hook and policy resources. Do not add a second Pluto registry merely to relay one synchronous call.
4. Freeze exact official hook/caller sources and manifest targets. Require actual selected-plan acceptance, one-context-call/source composition check, native type proof and bounded differential tests before claiming migration. This proposal does not itself authorize or prove ordinary-package activation.

## Required differential tests

- No-op policy / empty callbacks preserve exact original and current speed; native dispatch still runs once in documented order. A prior speed modifier and later trait observer prove current versus original semantics. Verify old host/client injections are absent so the shared route does not run the policy twice.
- Same actor inputs reach the same policy through local, swapped guest, remote prediction, guest STOP and delayed paths; another player's equipment/frame cannot leak. Mismatched remote frame fails closed. Use explicit event pos even when game.mining differs or is None.
- Non-TConstruct held item still receives valid armor multiplier; malformed/missing custom_data/Tool, broken flag, durability edge and unavailable Broad stay distinct. Source modifiers, levels/order, conditional effectiveness and armor order remain exact.
- Current water and previous water disagree in both directions; Aqua Affinity present/absent, absent player and ground/air, Haste versus Conduit max, every fatigue branch, and independent native attributes. Preserve the existing flat-bonus oracle rather than re-deriving all bonuses from current/original ratio.
- Hardness zero with speed positive/zero/negative, NaN/infinities, ordinary positive hardness, -1 unbreakable, creative route. Check arithmetic classifications and whether native break threshold is reached, not an invented sanitized speed.
- Native block/held/facts and Items base evaluation remain intact. No callback sends packets, damages tools, breaks blocks or mutates progress. Computation may run more than once per tick through native paths.

No Rust, Cargo metadata, Mixins plan/apply or tests were run for this review. No compatibility claim for latest or full TConstruct is made.
