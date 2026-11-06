# Existing extension points and the remaining contact/effect contract

**Checkpoint proposal, not enabled functionality.** This source-only follow-up
uses official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. No native repository is changed.
The existing negative state proof remains in [contact-effect-attachment.md](contact-effect-attachment.md).
This document proposes the next boundary to the integration owner; it does not
claim that the proposal is implemented, approved upstream, or runtime-tested.

## Existing facilities that were actually checked

1. **Typed attachments are real reusable storage.** `AttachmentType<T>` and
   `Attachments` support owned cloneable values, optional codecs, preservation
   of unknown saved entries, explicit no-default reads, player respawn copy and
   tracking-client sync. `Holder` covers a level, chunk, block entity, entity or
   player. It has no effect-instance holder. Access through Host requires a
   mutable host; a static contact or read-only effect-tick callback cannot
   acquire it. Entity attachments also fail when the entity is not in the
   current owner's lists. This is useful for independently entity-owned mod
   data, not an identity map for native effects or their hidden chains.
2. **Effect callbacks have deliberate limits.** `on_effect_added` and
   `mob_effect_added` provide effect ID, actor identification and amplifier,
   after the native addition. They do not provide the incoming/active instance,
   an instance token, curative policy or a hidden-chain transition. The first
   only fires for a new effect kind. The tick callbacks use a read-only Game;
   official documentation explicitly says that Game does not hold the ticking
   mobs or the player's effects at that point. The mutable `keep` flag can end
   that current mod effect during its native tick, but is not an item-cure or
   general effect mutation interface. Vanilla effects do not enter these
   content-mod callbacks.
3. **Custom consumption has a genuine mutable acting-player context.**
   `native_consume_effect(kind, data)` is called for an explicitly mod-defined
   consume effect and has `&mut Game`. It is a useful future caller for a generic
   cure service once native instance policy exists. It is not a hook for every
   vanilla ClearAllEffects operation and does not itself receive the consumed
   stack. The exact pre-consumption stack is available in `finish_consuming`,
   before its ordered effect loop and before held-stack decrement/remainder.
   A cure API must receive that actual stack identity, not reread whichever hand
   happens to be selected later or change every native clear operation.
4. **Entity/living handles validate an entity, not an effect lifetime.**
   EntityRef carries the native world epoch and entity ID; lookup checks the
   active native world/roster. LivingRef narrows supported kinds and currently
   exposes equipment operations. Neither supplies a native effect-instance
   handle. A valid entity handle also does not grant a mutable borrow while
   simulation owns that entity. It must not be synthesized from contact IDs to
   bypass the missing same-invocation capability.
5. **NBT merge is not an effect attachment mechanism.** `native_entity_nbt`
   reads the entity's saved representation through a validated handle.
   `native_entity_merge_nbt` reloads supported entities and explicitly refuses
   players and guests. It cannot transfer runtime instance metadata through
   ordinary updates or hidden restoration. Rebuilding a mob from NBT during a
   contact would be a different operation with broader effects, not a cure.
6. **Mixins bridges expose existing members, not new ownership.** GET/GET_MUT,
   SET and INVOKE preserve Rust borrowing and dependency direction. They do not
   add storage or lifetime identity, cannot invent a missing Game borrow, and
   do not bypass the observed EffectInstance state rejection. A bridge to
   downgrade/update would still need a real metadata owner. Generic owner
   types are outside the documented bridge contract.

The checked official catalog contains 432 hooks and 174 declared helpers. The
relevant entries and exact source hashes are recorded in
[effect-extension-inventory.json](effect-extension-inventory.json). This is a
scoped inventory, not a claim that arbitrary new source transformations could
never be written.

## Narrow proposal for the integration owner

The missing guarantees belong together. Solving only serialization or only an
extra callback would leave the original TConstruct behavior incomplete.

### A. An owned native instance extension contract

Ask for a supported extension payload owned by each real native effect instance,
with a public accessor/constructor contract compatible with existing callers.
Do not add a forbidden private Mixins state field or change an attribute solely
to get past its planner. Do not key it by address, collection index or effect ID.

Reuse the existing attachments' namespacing, codec validation and unknown-data
preservation where practical. Their current holder APIs cannot simply be
relabelled: Attachments has pending holder-sync and respawn-copy state, and no
PartialEq implementation, while EffectInstance derives PartialEq. Equality,
copy and wire semantics must therefore be decided explicitly rather than
embedding that type blindly.

Required instance transitions:

- New native construction starts with an explicitly documented empty/unresolved
  extension state; a mod can initialize its own policy before insertion.
- A whole instance clone and a hidden-instance copy duplicate owned metadata.
- Ordinary effect update retains the active policy, as the inspected Forge
  source does; hidden construction copies the actual previous/incoming instance.
- Restoring the hidden instance copies its metadata with its native details.
- A durable effect record saves each instance's payload and hidden chain, with
  backward-compatible defaults and preserved unknown namespaces. The current
  five-field player save and mob tuple are insufficient for that contract.
- Removal, expiration, death, replacement and unload drop the corresponding
  owned data. Respawn-copy is a separate policy, not an ordinary instance clone.
- The authoritative server remains the decision maker. Any client-copy or wire
  policy must be explicit; missing metadata in the current native effect packet
  cannot be treated as authority to remove or cure the server's instance.

Player effects and mob effects need separate implementation/proof gates until
they share an equivalent record/lifetime model. The current mob `hidden_effects`
list only hides particles; it is not a suspended weaker-effect chain.

### B. A same-invocation contact capability

Keep the existing read-only contact event for observers. For effectful behavior,
add a separate scoped interface constructed by the code that actually owns the
current actor. It must expose only the necessary operations and must not return
a global Game reference or retainable mutable actor pointer.

The caller must supply the correct world/dimension and live actor, the exact
contact position/order, and the applicable earlier fluid-update snapshot. The
snapshot has to be owned by that actor or its current simulation invocation,
not reconstructed later from roster bounds. Missing/stale/unloaded/budget-failed
facts remain unavailable, never silently dry. A player implementation must
cover both local simulation and the swapped guest-server path before claiming
player parity. Nonliving item/projectile contacts must not acquire an effect sink.

Effect addition must complete while the actor is still borrowed at the original
contact site, before proceeding to the next native per-block action. A callback
may return a bounded request that the current owner applies immediately, but a
later tick queue does not satisfy this requirement. More complex source behavior
such as fire, damage and conditional sound needs the result of each operation in
its original order; an unordered batch is not equivalent. No owner is inferred
when the source caller supplied none.

### C. Item-cure entry, separate from clear/remove operations

Once A exists, use the actual consumed stack from `finish_consuming` and the
explicit acting-player context. A generic service can inspect each real active
instance's curative policy and perform only accepted removals with native status
updates/callback order. `native_consume_effect` is an existing valid entry for a
mod's own consume kind. Supporting source vanilla-milk curing additionally needs
an explicit native consumption boundary; do not globally redefine
PlayerEffects::clear, commands, death, expiry or arbitrary ClearAllEffects data.

TConstruct continues to supply its fluid-to-effect definitions, durations,
empty curative lists, immunity and burn/sound policy. PlutoLib owns bounded
facts, namespaced instance-data access, supported host adapters and reusable
operations. No TConstruct ID or default milk policy belongs in generic storage.

## Smallest validation sequence before activation

1. Compile the supported owned payload with every native constructor and
   cross-crate struct-update caller. Check independent clones, ordinary refresh,
   stronger/longer hidden cases, downgrade and complete hidden-record roundtrip.
   Reject malformed/oversized metadata before mutation; preserve unknown saved
   entries and old-save defaults.
2. Prove a no-op scoped contact adapter against the exact complete official
   plan and applied types. Show native callback order, stable same-actor context,
   local/guest handling and missing-snapshot failure without changing vanilla.
3. Exercise a bounded real-host contact/consume scenario with two instances of
   the same effect ID but different curative policy, including a hidden restore.
   A potion-created instance must not inherit a fluid-created instance's policy.
   Verify two actors/dimensions, save/reload, client non-authority and unrelated
   clear paths. Pure DTO tests alone cannot establish these facts.
4. Enable only the proved actor domain and explicit consumer path. Keep other
   domains unavailable rather than silently using the entity attachment map.

No existing facility inspected here currently supplies all A/B/C guarantees.
The recommendation is to agree this official extension boundary first, then
implement its reusable adaptation in PlutoLib. No official-repository publication
or runtime activation is performed by this checkpoint.

## Source references

- [Attachment holders and values](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-world/src/attachments.rs#L43-L145)
- [Attachment host ownership](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/mod_api_host.rs#L97-L150)
- [Official attachment contract](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/docs/native-mods.md#server-attachments)
- [Effect-tick borrow limits](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/native_hooks.rs#L7728-L7800)
- [Ordered consumed-stack context](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/consume.rs#L44-L97)
- [Entity NBT limits](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/native_entity_data.rs#L14-L59)
- [Supported bridge contracts](https://github.com/horizon-rust/horizon-mixins/blob/e8cd0aa38cffef49cb2e088ef716baa107148c02/docs/scopes-and-bridges.md#reusable-field-and-method-bridges)
