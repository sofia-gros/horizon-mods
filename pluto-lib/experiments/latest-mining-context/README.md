# Latest mining context: unapplied native-phase candidate

This is a source-only response to the actual `E_LOCAL_NOT_FOUND` at `source-host-mining-context`, `Game::tick_mining`, `speed:0`. The preserved 600-second diagnostic is `fb96cb825a5cb54b42a5914923e823ee09289890`. No original main, PR, package, old proof, official source, CLI or ordinary Pluto bundle is changed here. No planner, apply, Rust type check or gameplay test has run for this candidate.

## Boundary and ownership

Latest official Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd` delegates host mining to `hand::Game::destroy_progress`; the latter invokes `native_break_speed` after native environmental speed calculation and before hardness/harvest division. That official owner already supplies a synchronous, generic operation boundary for host, acting guest and remote prediction. Its file documents Mixins HEAD before the trait-mod dispatch.

The candidate replaces the two legacy lexical speed captures with one HEAD at that official target. Formal `&Game`, position, state and mutable current speed are borrowed. A new private Pluto module resolves facts from the formal state and borrows the same actor's held stack and pos/max_y. The TConstruct consumer passes those exact references to its existing shared policy once. It does not read `game.mining`, reconstruct a stack, mutate original, cancel dispatch, or repeat the native calculation.

The operation and its host/guest/client dispatch belong to Horizon. The new `game::pluto_mining_context` internal support module owns only native fact acquisition. Mixins supports `pub(crate)` (the default) and `pub` module declarations, not a private declaration; this candidate explicitly selects `pub(crate)`. Its `pub(super) BorrowedFacts` and `pub(super) read` are internal Game-side facilities, not a new public authoring API or registry. The reader returns `&StateFacts`, `Option<&ItemStack>`, formal pos and max_y; it neither validates actor/slot authority nor projects components or clones retained Wire. The formal state/position association is the official caller's responsibility. All current-speed policy stays in TConstruct's existing shared body, including material rules, Haste/Broad snapshots and matching actor-wire authorization.

Actual archive 4f13 already contains `pluto_tool_rules` and its Items speed/drop RETURN hooks. They remain byte-identical and are the separate base Tool stage, not an actor-context API. This candidate does not call an imaginary installed service or move consumer policy into Pluto. The newly added private module is explicitly uninstalled and untyped until later real gates. Existing public APIs stay unchanged.

## Exact candidate delta

`candidate.py` has no command entry point and never writes files or starts tools. A later independently reviewed runner may call its pure bytes-map transformation after recreating and verifying the previous revision-only candidate:

- Input: the exact 644-member revision-only main 08afa + common 4f13 payload, SHA256 seal `692fa172e45e00dcddf62d201c6dbf206702f16e8b06a7109c8e6b6c13f0c125`.
- Replace only `tconstruct_native/horizon-mixin.toml`: remove the adjacent `source-host-mining-context` and `source-client-mining-context` injections, insert `source-native-mining-context` HEAD in their place. Other hooks preserve order, attributes and bytes. Main hooks become 289 instead of 290; common hooks are unchanged.
- Add `tconstruct_native/source-mining-context-native.rs` and `pluto_native_creative/mining-context.rs`. Append one explicit crate-visible internal module declaration to the candidate common manifest. The original two unused helper files remain byte-for-byte as historical inputs. Exactly four candidate paths differ; payload count becomes 646, and common modules become 70 instead of 69.
- Shared policy, rules/math, both existing Items::mining_speed RETURN owners, other modules/state/resources and all remaining payloads remain byte-identical. Whole-map inverse restoration removes both new source files, removes the appended private declaration and restores the exact two old hook blocks.
- Both original archives remain immutable. The new common candidate is not the old 4f13 proof. The prior two revision-only changes remain separately identified. Raw package headers keep old pins, so this is not a repaired loadable package or a compatibility declaration.

`oracles.json` contains the exact main 08afa manifest (compressed), old hook blocks and two old adapters. Source and packaged-policy hashes are separate: the existing main packager changes native mod id/accessor prefixes. This candidate preserves that already verified packaging transform; it does not mistake raw source hashes for packaged bytes.

## Order and limits

HEAD means TConstruct policy runs before official ordered trait dispatch. The immutable `original` argument stays native's incoming value; only current speed changes. The old official version lacked this trait event. This is an explicit new composition contract, not blanket old/new multi-mod equivalence. Removing both old context hooks avoids a duplicate callback when latest host mining delegates to the shared method. The separate Items RETURN base Tool/broken-state stage remains.

The unchanged policy still applies Armor Haste to non-TConstruct held items, distinguishes malformed/missing Tool and Broad evidence, uses matching remote Haste/Broad frames, and separates current eye-water from previous-eye Hydraulic. Native attribute/environment arithmetic is not replaced by a current/original ratio. Formal pos also works for guest STOP/delayed queries whose `game.mining` may differ or be absent.

Official native differences are not sanitized: old host hardness-zero special handling differs from latest common division; zero/zero yields NaN. `tick_mining` tests `>= 1`, whereas `guest_delayed_destroy` tests `< 1` then takes its completion branch, so NaN has different caller consequences. Negative/non-finite values, water/ground operation order and native hardness restrictions are not changed or claimed equivalent. The callback may run more than once per tick and grants no write, save, notification, item consumption or block-break authority.

## Verification

Run `PYTHONDONTWRITEBYTECODE=1 python3 test_candidate.py` for the pure source/manifest guards. These checks do not execute the Rust policy or prove borrowed Game types. All 16 pure guards and four exact latest official source/hash comparisons passed. Full 644-to-646 payload transformation, actual selected plan, private-module/helper visibility type check and actor/environment differential tests remain unexecuted. Recipe's independent source report and test requirements are included under `review/`; they are source review, not runtime proof.

Before any later execution, freeze and review the new runner, preserve the prior refusal, verify the four exact official files, keep existing safety guards and serial release, and stop on the first new diagnosis without broad capture changes. No existing 12 common guards are weakened.

## Next execution options, not authorization

The prior full candidate needed 575.217 seconds before its first old-hook refusal. Removing that refusal does not guarantee a complete plan inside the unchanged 600-second cap. A bounded first probe can supply only the exact two new source files, the internal module declaration and one HEAD, on the exact latest official source. It can prove target/formal-binding/owner acceptance, not full main/common compatibility or unresolved TConstruct policy types. Do not replace the policy with a stub and call that the actual adapter proof.

A separate real type gate would need the actual applied Game and private Pluto module to prove native borrows/visibility, then the complete candidate to prove its TConstruct policy wiring. No local Game model can replace those proofs. The eventual complete 646 payload still needs its own one-shot 600-second plan and unchanged 12 guards. A timeout remains unjudged; subsequent refusals are preserved without automatic retries. Rendering's optional exact witness collector belongs only after accepted complete plan and a real plan file; it is not reached after a rejection and does not authorize guard updates.

Visibility correction after source checkpoint6c0a: the generated module is explicitly `pub(crate)` while its struct, fields and read function remain `pub(super)` to the Game parent. This does not make them an external mod API; actual cross-child helper visibility still requires the native type gate.
