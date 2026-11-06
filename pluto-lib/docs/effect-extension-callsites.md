# Native effect extension: concrete implementation inventory

**UNFINISHED and UNAPPLIED.** This is an upstream-facing proposal stored only in
the mod repository. It neither changes an official checkout nor enables a
PlutoLib hook. The fixed targets remain official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`
and Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`.

The companion [machine-readable inventory](effect-extension-callsites.json)
records file hashes, exact source anchors, and all lexical constructor matches
in the two inspected gameplay crates. Matches include tests and declarations;
their count is not a claim about AST constructor coverage.

## There are three distinct native effect representations

1. The actual player path uses `horizon_client_render::effects::EffectInstance`.
   It is an owned value with a derived deep `Clone` and `PartialEq`, and an owned
   boxed hidden chain. `PotionState.effects` and each LAN guest's `effects` own
   `PlayerEffects` collections of those values.
2. The actual mob path uses effect tuples plus particle-visibility lists. It
   does not have the player's hidden-instance chain. It remains a separate
   migration, not a second name for the same storage.
3. **An additional, useful official kernel already exists:**
   `horizon_world::status_effect::EffectInstance<H>` and `ActiveEffects<H>`.
   Here `Clone` shares an `Rc<RefCell<EffectData<H>>>`; `copy_instance` creates
   an independent copy without the hidden chain or blend state. Equality compares
   the effect holder and details, not identity, hidden chain or blend.
   `TickWorld`, `EffectHooks` and `StoredConsume` provide genuine instance-aware
   callbacks and collection access to their explicit owner. These are better
   foundations for a future unification than a new side table.

The third representation is not the inspected gameplay storage. A scoped search
found no `status_effect::` or `ActiveEffects<` references in the client or render
source trees; the actual owner fields and call sites above confirm their current
concrete types. This is not proof that every possible alias is absent. A generic
kernel fixture must not be presented as a live player/mob fix.

## Smallest bounded prototype choice

First propose an owned extension payload on the **existing player value type**,
without changing its established clone semantics. It must remain an unapplied
prototype until native constructor, save, authority and lifecycle gates pass.
Do not migrate the whole gameplay owner to the shared-reference kernel merely
to obtain a new field; that would change aliasing, equality and callback behavior
and require a much broader review.

An upstream-supported public payload field with a validated payload type would
allow the two existing cross-crate struct-update callers to keep using
`..EffectInstance::new(...)`. A private field or `non_exhaustive` change requires
those callers to switch to supported builders. Either option is a deliberate
upstream API change, not a way around Mixins' observed state rejection. Full
external struct literals are a compatibility consideration even though the
inspected native callers use struct update.

PlutoLib would adapt that supported payload and own its own namespaced encoding.
TConstruct would still select the empty curative list and effect behavior.
Neither effect IDs nor implicit milk defaults belong in generic native storage.

## Exact ownership and copy changes to examine

- `EffectInstance::new`: explicit empty/unresolved payload, not a copied global
  policy. Native amplifier clamping stays unchanged.
- Derived `Clone`: every value and hidden instance gets an independent owned
  payload. A reference clone from the world kernel is not interchangeable.
- `update`: stronger or same-level refresh retains the active payload. A newly
  hidden previous instance copies the previous payload; a newly hidden weaker
  incoming instance copies the incoming payload. Updating an already hidden
  instance follows its own ordinary-update rule.
- `downgrade`: the restored instance's payload must move with its details and
  remaining chain. Leaving the expired active payload behind is incorrect.
- Beacon and conduit struct updates (`game/beacon.rs`,
  `game/block_entity_anims.rs`): remain new instances with explicit default
  payloads unless a real caller supplies one.
- `PotionState::swap_player` and both calls in `lan_actions.rs`: move the actual
  guest collection into and back out of the active game context. No effect-ID
  map or equipment DTO may substitute for that ownership.
- `tick_mob_effects`: temporarily takes the entire collection before effect
  callbacks. A callback with `&Game` cannot borrow that absent collection.
- Removal, expiration and death drop the owned value. No cleanup side table is
  required. Item cure remains separate from death/commands/arbitrary clears.

## Persistence is not solved by adding a field

`PlayerEffects::saved/from_saved` and `save::Level.active_effects` currently pass
five-field tuples. `Level::read_player/player_tag` encode/decode those tuples.
This loses the native hidden chain even before extension data is considered.

A durable implementation must change the record across all of these routes:

- Local save collection capture in `game.rs` and `restore_effects`.
- LAN `playerdata` load and save, which use the same Level record.
- `Level::read_player` and `player_tag`, including signed-byte amplifier parsing,
  particle defaults, old saves and effect-list omission when empty.
- Saved hidden records, their unknown namespaced payloads, malformed input,
  resource limits, and independent reconstruction of each instance.

Do not add an ID-keyed side channel beside the tuple. A typed durable record
should own each payload and its hidden record. An upstream choice is required
for the record/API and failure policy; a prototype must label any chosen bounds
and codec as proposals rather than pre-existing gameplay semantics. No lossy
fallback may silently claim persistence success.

## Network and cure are separate authority gates

`remote_effect` replaces a client instance from a packet containing only native
effect details. That replacement must not invent server curative authority or
inherit the replaced instance's server-only payload. Server-side ownership can
remain authoritative without inventing a protocol extension; client-side cure
decisions must remain unavailable unless a supported wire contract exists.

`finish_consuming` has the actual consumed stack before the ordered consume
effects, held decrement and remainder. `ClearAllEffects` is handled separately
in `Game::apply_consume_effect`. A future item-cure boundary must supply the
actual stack and active actor at that point, while leaving other clear callers
alone. Filtering and removal notifications must be defined with the native
owner, not implemented as clear-and-readd.

## Contact remains independently blocked

`MotionWorld::check_inside_blocks` owns the live motion/world borrow and emits
the contact inside the per-block loop after its deduplication test. It does not
own PlayerEffects. Its `block_contact::emit` arguments contain exact position and
state, but the enclosing loop uses destructured locals. Mixins' documented
plain-let capture support cannot simply be assumed to capture those patterns.
A read-only identity-preserving argument adapter may be investigated, with an
explicit receiver borrow and actual planner/type proof. Even if that succeeds,
it supplies observation, not the earlier immersion snapshot or effect authority.

## Validation status and next gate

This unit is a source inventory only. The official checkout remained clean;
JSON syntax and whitespace checks are local documentation checks. No native
plan, apply, Cargo execution or runtime activation was performed.

The next permitted artifact is an explicitly unapplied prototype and bounded
copy/hidden/save-contract tests. Its test report must distinguish tests against
real selected native source, proposed changed source, and any unimplemented
save or authority integration. It cannot certify live contact effects, LAN,
mob parity, production saves, or upstream acceptance.
