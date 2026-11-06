# Immediate guest contact: verification plan for an unapplied native extension

**UNAPPLIED / NOT EXECUTED.** This is a validation plan, not a production hook,
frame implementation, runtime result or permission to start a native job.
It builds on the saved [native refactor design](synchronous-contact-native-refactor.md),
[borrow phase map](contact-borrow-phase-map.md), exact Game compilation probes
and [guest observation proposal](guest-continuation-observation.md).
The accompanying JSON freezes 19 official input hashes, 11 existing artifact
hashes, six dependent gates and 21 proposed cases, all native cases **not run**.
The original prototypes are unchanged.

The source target remains official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`, Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. No official checkout or abandoned
fork is changed or used. Existing source checks and the earlier Pluto package
proof are not relabelled as proof of this extension.

## Decision and smallest useful candidate

The first useful candidate is still **one server guest player**, inside the
existing `guest_server_tick -> as_guest` turn. Initial positive fixtures require
a genuine loaded, living, unmounted guest in an admissible Ready server phase,
without an outer active guest turn. These are bounded fixture preconditions, not
a new general permission rule. Preserve the original static
Inside notification. Suspend the motion traversal immediately after the
accepted hit's visited insertion and that notification, **before** its native
per-block action. End every MotionWorld and borrowed resource/definition view,
let the real upper Game owner perform the admitted operation, then resume that
pending native tail exactly once under an independently established continuation
contract. No delayed event queue is an equivalent replacement.

The first implementation milestone would be a **no-op traversal refactor**,
not a new mutable callback API. The later candidate operation is one closed,
explicit native effect operation, not arbitrary user closures with mutable Game.
Its closed call graph and allowed state changes have not been established.
Builtin IDs avoiding the content-effect callback branch are only a promising
restriction to investigate; they are not sufficient admission by themselves.

Current verdict: a lexical split is plausible and has an exact source location,
but an immediate effectful extension is **not yet demonstrated implementable**.
The existing prototype establishes neither the real frame's compilation nor
safe continuation. Keep production activation blocked rather than inventing
teleport, block-mutation, error or curing behavior.

## Important correction: the frame crosses a real crate boundary

The previous design's phrase “private owned frames” describes encapsulated
state, not a sufficient Rust visibility implementation:

- `horizon-client-lib/Cargo.toml` selects
  `../horizon-client/src/client_lib.rs`; that root owns `pub mod motion`.
- `horizon-client/src/lib.rs` reexports the lower crate and owns `pub mod game`.
  The private Game is therefore in a **different, higher crate**.
- A `pub(crate)` motion frame/method alone is inaccessible to that Game. The
  exact future candidate must either supply a deliberately minimal cross-crate
  opaque frame surface with private fields, or demonstrate another legal
  placement. This plan selects no reverse dependency and no copied MotionWorld.

An opaque lower frame may represent movement cursors, hit snapshots, visited
state and a pending native tail. That data is **not** a public actor capability,
proof of a valid owner, or a transferable right to call Game. The private upper
driver owns the actual turn and its admission. All lower frame exposure and
reuse rules must be part of the real two-crate compile/API review. The existing
Game-only sequential-borrow probe cannot establish that additional boundary.

## Two independent continuation problems, plus the enclosing caller

1. **Who may continue?** The native body, live guest turn, Ready phase and
   relevant resource owners must still be the ones the operation is allowed to
   affect. Numeric entity ID, equal coordinates, world epoch, a read-only Actor
   DTO or `Some(player)` do not prove that. A same-ID body replacement and an
   out-and-back teleport are deliberate adversarial cases.
2. **What may the traversal read?** `check_inside_blocks` explicitly assumes
   “Nothing below changes a block” when it collects hit states before iterating.
   Later per-hit shape/context queries still occur live. A mutable callback can
   invalidate that assumption **without changing the actor**. World mutation,
   shape changes and resource reload need their own specified snapshot/live-read
   contract. Do not choose stale states, reread everything or restart as a
   convenient default. A body-generation token alone would not solve this.
3. **What may the surrounding tick retain?** The upper guest caller carries
   earlier rules, movement and flags through later environment/effect/travel
   phases. Cancelling only the inner frame after a callback does not define the
   rest of that caller. `server_block_damage` also propagates an inner error
   before freezing, while its caller uses `let _ =` and continues. Preserve both
   control-flow levels on the no-op path.

For a narrow supported operation, an exact closed source/installed-plan audit
might show that invalidating operations cannot occur. That is a prospective
alternative to a general continuation capability, **not a proven property**.
If that cannot be established, a coherent native owner protocol is needed.
Rejecting the extension before activation is safe; performing a mutation and
then selecting an unspecified recovery action is not rollback or parity.

## Six gates and explicit acceptance limits

### G1. Existing real Game compile probes

Use the CI worker's unchanged positive/negative sources and strict diagnostic
validation. Positive acceptance proves sequential field reborrowing; the
negative must fail specifically at its designated overlapping whole-Game call
with E0499. An unrelated compiler/environment error is not success.
This gate is prepared but has not executed. It does not run a contact, move a
player or prove that the actor before and after is the same live body. The new
plan does not add a job, launch CI or broaden the existing CI scope.

### G2. Real cross-crate no-op frame overlay

After G1, prepare an exact, isolated, test-only overlay that implements the four
existing frame responsibilities: server damage phase, movement/visited/group
phase, hit cursor, and consumed-once native tail. Compile both actual crates and
the private guest driver. No substitute Game, copied native structs, unsafe
reborrow, pointer/ID side table, static mutable owner or new reverse edge.

The original native path remains separately available as an oracle. The no-op
mode must not introduce TConstruct, cloud insertion, renderer/provider changes
or a new authority registry. Public opaque data is not authority. Compilation
alone does not prove no-op behavior or frame lifetime admission.

### G3. Actual native no-op equivalence

Run original and candidate against independently initialized, genuine native
setups using the same recorded resources, world input, guest setup, seed and
controlled time. Game is not assumed Clone. Both sides use identical minimal
trace instrumentation, whose exact diff is reviewed and hashed. Do not generate
the expected trace by calling a proposed duplicate implementation.

Record ordered native milestones rather than a sorted contact set: movement
record/axis index, budget charge, captured hit state, shape-read point, visited
insertion, original Inside, pending native tail entry/exit, errors, group freeze,
outer phase and guest leave/flush. Record actual effect/status observations
only where that gate needs them. Trace buffers are bounded; overflow fails the
fixture rather than silently truncating both traces to matching prefixes.

Cases N01–N08 cover stationary and repeated segments, budgets/endpoints,
air/shape exceptions, powder-snow grouping, native errors, deferred rose order
and out-of-scope paths. Compare only the declared contact/native-tail and final
actor observations; this is not a full Game, network or rendered-world oracle.

**Resources cannot be optional.** The official motion tests return early if
`movement-properties.nbt` is absent. They are useful source/setup examples but
their unmodified green exit is insufficient for this gate. The new runner must
require exact resource inputs and explicit executed case IDs/counts. Missing
resources, missing guest, unavailable native shape/error setup or zero executed
cases are blocked/failure, never skipped success. Tiny real-loader fixtures
prove their stated data only; production asset coverage stays a separate claim.

### G4. Admission and continuation closure

Before adding an effectful operation, close both actor-owner and world/resource
contracts for that operation and its complete installed source/plan. Preserve
all of `add_player_effect`: collection add/merge, `apply_effect_status`, absorption
handling and native effect callbacks. The status path touches modifiers, health,
armor protection and caches; a builtin ID excludes only a particular callback
branch, not all relevant code or other installed Mixins.

A01–A05 require actual roster/Ready entry, real native resolution with None
preserved, complete operation closure, and invalidation cases: teleport,
replacement/removal/respawn, nested guest, changed block/shape/resources and
wrapper errors/notifications. Unsupported transitions must be excluded before
an extension is enabled or governed by a separately implemented native contract.
The plan deliberately does not assign “abort,” “resume,” or “retry” as the
post-mutation outcome for those unresolved cases.

A normal closure Err still reaches the official explicit leave. Panic unwinding
need not; a future panic experiment uses an isolated disposable process, not a
catch-and-reuse of Game. Observe outer state after the entire leave and teleport
notification flush, not merely after the inner closure returns.

### G5. One immediate complete native effect operation

Only after G4, use the actual native operation, not list-only mutation or a fake
callback. Required trace: accepted original Inside -> operation begin -> full
operation end and derived-state observation -> pending native action -> next
contact. Verify exactly-once placement in the same native guest turn.

I01–I02 cover native new/refresh/stronger/weaker-hidden/unchanged outcomes and
preserve later tick ordering. An unchanged add result still goes through the
native status update. Use native canonical IDs: `EffectInstance::new` stores
its input and does not normalize `minecraft:` names. Comparison against a
separate native control insertion establishes only the bounded effect operation.
It does not supply TConstruct's empty curative list or cached immersion timing.

### G6. TConstruct guest-only content contract

The source `MobEffectLiquidBlock` needs positive cached fluid-type height,
a living entity, a fresh effect with explicitly empty curatives, then addEffect.
The installed native instance still lacks that ownership contract; the genuine
Mixins E_STATE rejection is not removed by a contact frame.

T01–T04 separately require source-authorized immersion timing and adjusted
bounds; actual instance clone/update/details-copy/hidden transfer; native old
save/local/LAN and malformed/unknown handling; actual consumed-stack cure
filtering with targeted removal/death/expiry left distinct. The saved consumed-
stack patch only transports an unused snapshot and still clears everything.
Excerpt/model tests and strict post-normalization NBT tests are not native
save/network/cure integration. Do not use effect-ID policy, restore-after-clear
or payload encoding into effect IDs to bypass that missing contract.

Only a later successful G6 could admit the declared **guest-player** consumer.
Local players need their own caller proof. Mobs retain a detached roster,
rider/navigation loans and tuple effects, so none of these player gates can
promote them. Items/projectiles are not LivingEntity targets.

## Execution, reporting and repository boundary

This unit writes only a MOD-repository plan, JSON and source/plan checks.
No prototype is installed, no manifests or production APIs change, and no
native build, metadata, planner, stage copy or runtime test is requested here.
Current capacity and coordinator serialization still govern any future job.
The continuous native floor remains **10 GiB + 128 MiB**, with measured headroom
and an explicit grant required. Do not force a local fallback while CI is pending.

For each future attempt, record exact sources, applied package/plan, compiler,
resources/setup and trace/result hashes, full source/lock/generated-file seals,
explicit case outcomes and minimum free space. Preserve failures and distinguish
`not_run` from `blocked_missing_fixture`, `failed`, and genuinely `passed`.
A complete-plan guard must bind the relied-on call graph, not only the contact
entry selector. A source-only hash check is not such an applied-plan proof.

Horizon and Mixins remain the host/loader. PlutoLib owns reusable adapters for
mods that choose it; TConstruct uses PlutoLib and retains its content policy.
Other Mixins mods may use Mixins directly. This plan neither publishes to an
official repository nor makes PlutoLib a required loader.

The companion source/plan suite checks crate topology, source order, native
resource-skip behavior, preserved prototype hashes and this plan's dependency/
case accounting. Its result is recorded separately from the unrun native gates.

Source/plan checks: **9/9 passed**, first in 0.019 seconds and again against
the finalized plan in 0.014 seconds. These were Python checks only; all 21
proposed native cases remain not run.

## G4 follow-up: concrete operation boundary, still no activation

This small follow-up narrows G4 using the saved official bodies and the existing
**ordinary PlutoLib v15 plan only** (runtime `f5a83539`, central `8715b123`,
published equivalent `89744814`). It is not the complete installed composition
of TConstruct plus arbitrary other mods. The JSON pins that plan's exact
1,048,672 bytes/SHA256 and eight additional source files. No new API, frame,
production guard or separate authority ledger is added.

The direct operation has five distinct phases:

1. `PlayerEffects::add` mutates the matching owned instance/hidden branches or
   appends an instance and conditionally increments its wrapping change count.
   Its mutable list borrow ends before Game calls `apply_effect_status`.
2. Status derives modifiers from the **entire current list**, compares them with
   `PotionState.applied`, removes/re-adds through live attributes, clamps health,
   and records the new applied cache. The caller ignores each modifier/clamp
   Result; partial failure must not be relabelled transactional success. Neither
   an incoming poison/blindness effect nor `changed == false` makes this a no-op.
3. Health/absorption work borrows actual PlayerMotion attributes, synchronized
   data and its events through `PlayerAttributeHealth`. The concrete `Updates`
   observer appends a health event; it does not receive Game. Live attribute
   dirty callbacks queue attribute bookkeeping. These are the inspected concrete
   paths, not a claim that every possible trait implementation is effect-free.
4. Protection cache reconciliation clones current armor, reads the active
   stations/items table (world tables or resource fallback), iterates damage
   definitions and evaluates native enchantment values with a local random
   source. It stores the keyed protection cache and replaces player status while
   retaining that protection. A future closure must include those dependencies.
5. The complete add operation then performs its explicit absorption top-up and
   `mod_effect_added`. The previous direct player borrows have ended before the
   server/content-effect branch calls native callbacks with mutable Game.
   Builtin IDs avoid this branch, not all earlier work or all installed hooks.

One concrete boundary needs more than a signature check:
`LiveAttributes::get_instance` may allocate through a boxed `Identities: FnMut`.
`PlayerMotion::new` supplies a native wrapping counter, while the public
`with_attribute_identities` constructor accepts a supplied allocator. A native
caller/setup proof must establish which implementation is actually present;
“the field is LiveAttributes” is not that proof. The direct bodies inspected
here contain no teleport/world-set call, but neither all transitive dependency
implementations nor the full installed call graph have been closed.

### Known overlap in the frozen ordinary plan

The plan has 34 injections, 66 modules, three states and two bundled crates.
None of its injections directly targets the 13 named add/status/update/attribute/
health/native-effect endpoints listed in the JSON. This is a finite exact-plan
inventory, not proof of an empty transitive call graph.

Do not turn same-file changes into false conflicts or false assurances:

- `effects.rs` has the shared `StatusFlags::protection` RETURN injection, not an
  injection into `PlayerEffects::status` or `EffectInstance::update`.
- `health.rs` has protection hooks on hurt/ArmorWorld's server-level observation,
  not `PlayerMotion::set_health`.
- `items.rs` has mining-speed/drop hooks, not this add/status endpoint.
- `game/potions.rs` adds `pluto_cloud_factory`; its source hash is recorded.
  A module declaration is not by itself an effect-add hook.
- All three recorded state targets are ChunkMap, not the player/effect structs.
  This does not say that another installed mod cannot change those structs.

Before any future G4 pass, enumerate and seal the **complete selected installed
composition** and the resolved call closure. Any unreviewed hook kind/selector
at a relied-on function, state/constructor change in relevant ownership types,
or namespace/import/module resolution change must fail the proposed admission
until explicitly audited. Mere absence of these 13 target names does not suffice.
Whole-Game callbacks for custom effects remain outside the narrow builtin
candidate unless they acquire their own genuine continuation contract.

G4 remains `blocked_design`. The next useful proof is still the already prepared
actual Game borrow gate, then the actual cross-crate/no-op frame and resource
fixture gates, not enabling a status-only shortcut. This source inventory runs
no native job and does not change the pending CI scope. The earlier independent
review found no serious contract defect in the plan; that was source review,
not a type/runtime/ownership result.

G4 follow-up source/ordinary-plan checks: **13/13 passed in 0.036 seconds**.
The frozen ordinary plan was read, not regenerated; no metadata, planner, apply,
Cargo, runtime or full-installed-composition check was executed.


## Fixture admission follow-up: real Game is necessary, not sufficient

This is a source-only refinement of G1 and the missing G3 runtime setup, not a
new gate, runner, public API or permission to run CI. The existing Game CI is
frozen at local `19059bb96ae954876ea99b7f8eb5e1b36cfbe42f`, published as
[`9385b77a37f19fab517b5a1ec09b4e1f8c824408`](https://github.com/sofia-gros/horizon-mods/commit/9385b77a37f19fab517b5a1ec09b4e1f8c824408).
Its exact probe and runner hashes are recorded in the JSON. It checks the real
host library's positive borrow and precise negative E0499 diagnostic; it does
not construct a live Game, load runtime resources, or include the guest observer.
That compile gate remains unexecuted in this evidence.

The pinned official sources do contain genuine CPU/headless Game setup paths,
but the existing tests are not interchangeable with the proposed fixture:

- `Game::new` initializes paused streaming, no player and default lifecycle.
  It can call `native_client_start` when `native_window()` is true; successful
  construction is not evidence of a server world-open transition. Headless
  `load_resources` selects the smaller `UiResources::server`, but still loads
  real core/world data. It does not make a resource-free runtime fixture.
- `native_entity_refs::client_tests` uses real resources and `Game::new`, then
  `connect_local` and remote state. Its scope is a remote client, not a server
  guest's mutable authority. It is ignored without prepared resources.
- `building_players::for_player_keeps_the_outer_body_in_its_original_dimension`
  also uses a real Game. It sets the role, supplies a player, starts `lan_host`
  and manually pushes a Guest with `placed` and `loaded` true. The test checks
  building boxes during a dimension swap, not a real world-open/guest-loading
  lifecycle. It does not drive `integrated_open` or readiness. Copying that
  setup and forcing a Ready flag would not close G3's admission contract.

A genuine future setup candidate exists in `integrated_open`: lifecycle opening,
world installation, mod startup, `lan_host`, `serving = Some(...)`, then spawn
settlement. Only after this succeeds does `run_integrated_inner` emit world
Ready and enter the existing command harness. The harness pumps the actual
`tick_server(true)` until a guest is placed and loaded, with timeout failure.
This is **not yet a prepared or executed fixture**. Resource identity, startup
callbacks and the complete installed composition still need verification.

Two specific traps must be rejected before adapting that harness:

1. The wait checks **any** placed/loaded guest. `harness_owner` later resolves
   `Serving.owner`, enters its level and guest, puts the host back in
   `Game.lan`, then invokes its closure. Our frozen observer instead requires
   `Game.lan == None` and no outer `lan_acting`. Calling it inside that closure
   is inadmissible, not an opportunity to clear fields or weaken its checks.
   A future test-only entry would have to run before the owner swap, select
   the actual owner, enter its real level and honor host/level restoration on
   ordinary errors. No such entry is implemented here; this is not a panic
   rollback claim. Preserve the existing leave/effect-sync/teleport/level-restore
   order when comparing any future adapter with `harness_owner`.
2. Role, readiness and contact scope are distinct. `as_guest_with_host` supplies
   a contact dimension only for `serving.is_some()` or `options.dedicated.is_some()`.
   Merely setting the IntegratedServer role does not establish that condition.
   Likewise the raw `loaded` flag differs from `Guest::client_loaded()`
   (`!dead && load_timeout == 0`), which is copied into PlayerMotion and also has
   timeout semantics. Native PlayerLoaded handling sets `loaded`, `loading` and
   `load_timeout` together. Neither flag alone proves loaded collision resources.

Before an authorized runtime attempt, record the exact resource/world inputs,
real world-open and guest join/load history, selected owner/body/level, Ready
phase, contact dimension and loaded shape data. Missing resources, the wrong
owner, a nested guest turn, unavailable collision data or zero executed cases
must fail or remain blocked, never become a successful skip. Observe outer
state only after the real leave/notification and level-restoration path.

Even a passing observer would only describe its selected native operations. It
would not implement the cross-crate suspended frame, close the complete native
callback graph, establish post-mutation continuation, or attach curatives to
native effect instances. G2-G6 and all 21 runtime cases retain their prior
unproved status. Mob paths remain separate. No official/production/probe source,
CI workflow or existing prototype is changed by this follow-up.

Fixture admission source checks: **19/19 passed in 0.061 seconds**, including
the prior 13 checks. The additional checks read eight pinned official files and
the four exact frozen CI files; both CI probe bodies match their unchanged saved
proposals. This was Python source/plan inspection only, not a compiler, resource
loader, network, native test, CI dispatch or runtime admission result.
