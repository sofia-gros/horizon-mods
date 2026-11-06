# Minimum persistence and synchronous contact contracts

**UNAPPLIED / UNFINISHED proposal.** Target: official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`. Nothing here changes an official
checkout, installs a hook, grants actor authority, or activates TConstruct.

## Persistence: a record codec is only one boundary

The added experimental codec is a standalone versioned NBT envelope for the
prototype owned `Record`. It has `version = 1` and an active-first `layers` list.
Each layer owns its ID, duration, amplifier, ambient/visible flags and opaque
namespaced extension byte arrays. The codec uses the actual official NBT types
and reader/writer, not a substitute Tag. Its exact source is pinned separately.

This envelope deliberately is **not** a replacement for `Level.active_effects`.
No persistent key, save-file placement or adoption by upstream has been chosen.
The separate add-on patch merely makes the proposed codec callable; it does not
connect native save callers to it.

The minimum codec guarantees are:

- Record-layer, per-layer entry, string/value and aggregate logical payload
  limits are explicit. Native NBT byte/node/depth limits are a separate input.
- Hidden order and every layer's unknown namespaced extension bytes survive a
  semantic record roundtrip. No registry lookup or actor insertion is performed.
- An unknown version, unknown schema field, invalid type, invalid UTF-16,
  noncanonical boolean, invalid amplifier, trailing bytes or oversized record
  fails explicitly. The caller still owns the original bytes/tag on failure.
  It must not replace an existing save record with defaults after that failure.
- The native disk writer intentionally writes overlong modified-UTF strings as
  empty. The proposal rejects those strings first, even if caller-supplied
  logical string limits would permit them. Merely checking UTF-8 byte length is
  insufficient for NUL or supplementary characters.
- Unknown **schema** fields are rejected, not confused with supported unknown
  **extension namespaces**. Native NBT parsing has its own normalization rules,
  including compound map semantics. The codec does not claim to preserve raw
  wire ordering, duplicate keys or arbitrary original bytes.

## Required legacy/native migration, still not implemented

The actual native player reader accepts numeric tags through `nbt_value::number`,
maps signed amplifier Byte values through `u8`, defaults missing duration and
ambient to zero, and defaults particles to visible. The actual writer emits
amplifier as a signed Byte after clamping, and writes `show_icon` equal to the
particle flag. It currently serializes neither hidden instances nor extension
payloads. The data fixer already knows the modern `hidden_effect` key, but the
player's five-field tuple decoder still does not retain that chain.

The proposal's strict Int amplifier and `visible` field are a **new envelope
schema**, not a claim that native legacy bytes are invalid. A legacy player tag
is explicitly rejected by this codec rather than silently misread as version 1.
The future migration needs a distinct, tested decoder for the original fields.
It must decide how to retain unknown vanilla fields and independent `show_icon`
values before replacing any old record. A payload-only patch does not supply
those missing native fields.

The full migration must cover local `sync_level`/`restore_effects`, guest
`lan/playerdata` load/save, `Level::read_player` and `player_tag`, the native
registry resolver, hidden chains and old-save defaults together. `player_tag`
first removes the old `active_effects` key from its preserved raw player compound
and writes the current tuple list. Keeping an extra raw tag elsewhere therefore
does not make today's save path preserve the new record.

Before commit to disk, the whole selected replacement must be decoded, validated
and encoded successfully. On failure, either abort that save with an explicit
error or retain the entire original record under a documented owner policy.
Do not mix successfully converted layers with a silently dropped remainder.
This is a required future owner contract, not an implemented atomic-file-save
claim. Native compression, filesystem replacement and local/LAN reload are
outside the standalone codec fixture.

## Contact authority needs more than a mutable effect collection

`Game::add_player_effect` does not only call `PlayerEffects::add`. Its exact order
is: record whether the effect kind was absent, add/update the owned instance,
apply effect-derived attributes/status, adjust absorption when applicable,
then dispatch the new-effect/every-add native mod callbacks. Those callbacks
take `&mut Game`. `apply_effect_status` also reads armor, enchantments, damage
definitions and its own cached modifier/protection state.

Therefore lending `&mut PlayerEffects` next to `&mut PlayerMotion` in
`MotionWorld` is **not** sufficient to implement the existing operation. It
would omit status/absorption and callbacks. Calling back into a global Game
pointer while MotionWorld holds its fields would not supply a legal borrow.
The current `Game` effect callbacks cannot be presumed to be read-only.

The exact inside-contact point is still within `check_inside_blocks`, after
its geometry and visited-position test and before native per-block actions.
For guest authority, `guest_server_tick` enters `as_guest`, samples base fluid
state and executes `server_block_damage` in that live context. Conversely,
`remote_player_tick` constructs MotionWorld for a client-side body, and prediction
replay also constructs MotionWorld. Constructor presence, a matching entity ID,
or a dimension string is not server mutation authority. Replay, remote-client,
collision-only probes, missing actors, and unsupported lifecycle states must
not acquire the proposed effect sink.

## Smallest viable upstream ownership change to evaluate

The native owner must expose a synchronous operation boundary that can run the
**complete** addition/status/callback sequence in the correct actor context.
There are two explicit design choices, neither implemented here:

1. Refactor the native add/status/callback operation onto a supported scoped
   owner that really has all required resources and mutable callback access.
   Its callback contract must be explicit; silently weakening existing mutable
   Game callbacks to a reduced context is not parity.
2. Restructure contact traversal so its native owner can end the temporary
   MotionWorld borrow, immediately execute one contact operation with the actual
   Game owner, and resume the same traversal before the next per-block action.
   This needs exact continuation state, visited/order preservation, original
   fluid snapshot ownership, and defined behavior if the callback removes,
   teleports or otherwise changes the actor/world. It is not a later tick queue.

Neither option is made safe by returning a list of operations after the sweep.
Burning behavior needs sequential results for fire, hurt and conditional sound.
No entity-ID side table, fake capability, guessed creator, hidden reborrow or
client-authoritative effect write is supplied by this prototype.

The next contact proof should be a no-op native owner/lifetime refactor with
unchanged order and explicit local/guest/client/replay tests before any consumer
activation. That broader native change needs a separate unapplied proposal and
applied-host evidence if it is later authorized. This unit adds no such claim.

## Evidence

See [the pinned source anchors](effect-extension-callsites.json),
[additional source hashes](persistence-contact-minimum-evidence.json), and the
[codec experiment](../experiments/effect-instance-proposal/README.md).
Source preparation and tests are reported separately from actual save/contact
integration. The prohibited user Horizon fork is not a source for this work.
