# Cloud / language source admission for official 41f

**Source audit only. No guard values or runtime changed, no new native execution,
and no Windows / Game / UI result.** Official Horizon
`41f822b659789780f58d32c63ddfb6887ea1b8bf`, Mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57`.

The existing ordinary common bundle is `035273c4805d75111e528c8c26607fa3a8d1d5bf`
(archive `4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d`).
Its cloud complete-plan guard still pins official 624c. This audit preserves that
old guard and its successful historical evidence; it does not silently repin it.

## Concrete blocker and source findings

The guard checks every complete official input file before accepting the cloud
modules and exclusive authority/publication boundaries. **10 of its 17 hashes
now differ**, so the unchanged guard must reject a latest workspace. This is a
static guard-input blocker, not evidence that the Rust adapter itself failed.

The ten changed raw official files were retrieved at the exact latest commit.
Their byte lengths and Git blob IDs were recalculated without trimming or adding
newlines. The old inputs were read from verified official Git objects and each
matches the existing guard SHA256. The seven unchanged latest blobs match those
old bytes. The JSON records both identities, selected source spans and links.

Across **41 selected function/type/impl spans, 40 are byte-identical**. The only
changed selected function is `run_dedicated_inner`; its changes concern older
client configuration, combat settings and profiling. Its save/ready/tick ordering
is preserved. All **15 Game field declarations** directly needed by insertion and
owner validation are identical. All five packaged cloud module hashes still match
the old complete-plan metadata.

| Changed file | Protected boundary finding |
| --- | --- |
| `lan_actions.rs` | `enter_guest` and `leave_guest` are completely identical. Menu and interaction changes do not authorize a call while the roster hub is detached. |
| `lan_entities.rs` | `take_entity_id` and `lan_assign_ids` are identical. Network idle/bare flags and rider ordering changed outside these functions. |
| `mod_api.rs` | Handle fields and implementations, epoch and authority variants are identical. Trait-mod on/off dispatch was added; it is not actor authority or a reason to bypass native dispatch. |
| `commands.rs` | `Target`, `level_entity`, `target_level` and `target_position` are identical. Feedback and container-loot behavior changed elsewhere. |
| `integrated.rs` | Open, Ready wrapper, run loop and close functions are identical. The new asynchronous `post` API is not used as a substitute for same-invocation authority. |
| `dedicated.rs` | Run failure and close functions are identical. The inner run changed, but `save_game` → ready notification → tick loop remains in order. Held-player chunk loading does not replace the existing placed check. |
| `dragon.rs` | `Breath`, player/victim-related types and selected cloud tick/actor helpers are identical. Other dragon damage/flight changes remain native. |
| `potions.rs` | Cloud layout, owner and selected tick functions are identical. Drinking statistics and thrown instant-harm Enderman behavior changed outside these spans. |
| `game.rs` | Fifteen required field types remain exact. No copied Game constructor is introduced. The many unrelated Game changes still require actual composition and type checking. |
| `client_lib.rs` | Adds modules/re-exports, while native language/Text owner re-exports remain. Latest installed module identity must be obtained from the actual complete plan. |

The three direct authority files (`native_lifecycle`, `native_entity_refs`,
`native_level_refs`), `mod_api_host`, `commands/players`, renderer `world` and core
`potions` are whole-file identical. Thus the present private insertion continues
to be a plausible port without a new Rust policy: explicit owner, real epoch /
level / roster lookup, Ready authoritative server role, loaded center, checked ID
successor, construction and reserve before take-ID / assign / vector push.
Presence is still not permission to impersonate an entity. This source finding
does not certify all transitive behavior or an actual insertion.

## Smallest separate adaptation candidate

1. Preserve all five current module bodies and their disabled/unconnected consumer
   status. There is no identified direct boundary change requiring a new cloud API.
2. Only after reviewing this evidence, prepare a **separate latest metadata
   candidate**. Keep all 17 full-file pins, helper hashes, placement/visibility,
   protected types and exclusive target-prefix checks. Do not weaken the guard to
   selected spans or remove its old historical version.
3. Obtain the real latest complete installed plan, including TConstruct. Verify
   actual owners and all injection/state/constructor conflicts. Never reuse old
   node IDs or fingerprints as new evidence.
4. Run guarded apply and the actual latest host lib/bin type gate. Then execute
   the existing two lifecycle, two insertion-rejection and two native factory
   tests in an honest applied-host harness when granted. Real guest/world/cloud
   publication coverage is an additional gate, not implied by those helper tests.
5. Windows11x64 playability still requires the complete TConstruct build, resources
   and launch checks. This source audit is not that result.

## Language and fluid boundary

All eight checked loader/root/Cargo/lock inputs are blob-identical from f4dc to
41f. Of the earlier 33 selected language/fluid paths, 31 are identical. The two
changed paths are `native_hooks.rs` (screen background/blur/tile additions) and
`remote.rs` (skin publication optimization). Neither change adds synchronous
mutable fluid contact authority. Source identity does not transfer old native
proofs to the new entire host.

Ordinary language adapter / Key production bytes remain in common035273. The
merge helper14 success, Pack8 success and source-only root-key experiment remain
separate. The root runner is saved at `c415107bccac24a43091e4ca2796aca6dd304534`
without a trigger, fixed to f4dc, and is lower priority than the playable complete
composition. No locale priority repair, cache completion or retained UI refresh
is newly activated here.
