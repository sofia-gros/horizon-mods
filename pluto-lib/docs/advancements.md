# Advancement data and inventory events

The consumer-facing API is `pluto_inventory_api::advancements`, inside
PlutoLib's existing shared inventory API crate. It has no Horizon or consuming-mod
dependencies, item registry, component key policy or game-content registration.
Install the single unified PlutoLib package. Do not bundle another copy of this
crate or create another context registry.

## Inventory event activation

Installing PlutoLib alone leaves native inventory events unchanged. The native
adapter is default-off and returns `Continue` before component encoding,
snapshots, hash work or advancement state changes when no consumer opts in.

Call `pluto_inventory_api::advancements::request_inventory_component_events()`
during consumer initialization **before the first inventory event**. The request
is idempotent and lasts for the process; there is one shared atomic owner and no
runtime unregister/reset. `inventory_component_events_requested()` exposes the
state. Native packages already require a rebuilt/restarted process when their
composition changes.

Never activate from an item predicate: the original ID/count hash may skip that
predicate precisely when only metadata changed. Requesting late is outside this
startup contract. The caller remains responsible for its startup integration.
**Step 3 consumer activation remains pending.** The foundation does not install
TConstruct criteria or silently request activation on their behalf.

## Generic API contract

- `with_data(&key, owned_data, || work())` scopes data to a synchronous callback.
- `data::<Key, Data>(&key)` returns an owned clone only for the same key object,
  key type and data type in the innermost scope. Missing scopes, wrong types,
  other keys/threads and zero-sized keys return `None`.
- Nested scopes shadow outer scopes, including explicitly absent data. Returning
  or unwinding restores the previous scope. Nothing remains after the callback.
- `inventory_key(ordered_slots)` hashes exactly what the adapter supplies. Include
  empty positions, item identity, count and all components. It is a transient
  change gate, not a persisted ID or security hash.

Key and data types are `'static`; they are not tied to a particular host struct.
The scope does not prolong object lifetimes, mutate the key, return references,
transfer across threads, or cover deferred/async work. Address comparisons are
opaque identity checks with no unsafe dereference. The facade owns one thread-local
registry in the already-shared crate, not one registry per consuming mod.

## Official Horizon adapter

When requested, PlutoLib's native adapter attaches at `Game::check_inventory_trigger` through the
official Mixins cancellable HEAD boundary. The old closure-heavy body cannot use
OVERWRITE with the pinned transformer. The adapter retains the latest official
trigger's slot ranges, candidate index, authority gates, player predicate,
progress and reward path. It replaces only snapshot construction, the event key
and item-predicate invocation. A second replacement of this entry point requires
explicit composition; do not assume an unrelated early-cancelling hook combines.

Each authoritative inventory, armor and offhand stack is encoded using native
`item_data::encode_components`. The unchanged official `ItemView` and an owned
`Option<horizon_world::nbt::Tag>` travel together. During each view's predicate
loop that encoded snapshot is available through the generic API. After opt-in, the full
component snapshot participates in change detection, including metadata-only
edits. No inventory, component persistence, player save format or network
packet is changed. The native `AdvState` and LAN actor scope still own the event
key, so it is not a global player cache.

Only inventory advancement predicates have this adapter today. Loot item filters,
container locks, equipment/entity predicates and arbitrary standalone `ItemView`
calls have no metadata scope. Consumers must fail closed, not guess by item ID.
Full component encoding on the inventory poll is intentionally conservative;
its performance with large nested stacks after opt-in has not been profiled.

The adapter's dependency uses the existing client-library re-export. Consuming
predicate helpers in the client library use `pluto_inventory_api::advancements`
directly, hiding the host-qualified adapter path. Native target changes stay in
PlutoLib. The predicate facade below converts that private host snapshot into lossless
host-independent values before consumer callbacks run. The generic data scope
remains an adapter mechanism, not the recommended consumer component interface.

## Packaging and validation

`package_native.py` includes regular Rust modules under `inventory-api/src`,
including `advancements.rs`, without a separate filename allow-list. Archive
checks verify its exact bytes, unique crate identity and source references. The
generic API tests cover nesting, panic cleanup, type/key/thread isolation,
zero-sized keys, reentrant caller Clone/Drop implementations and metadata-only inventory changes. TConstruct's bounded
predicates have separate tests using this shared facade, without inventing an
`ItemView.custom_data` field. Official focused apply/type-check results are
recorded in [the focused validation report](advancement-validation-20261105.md); they do not certify all other native hooks.

## Stable item-predicate callbacks

`pluto_inventory_api::advancements::predicates` owns the single process-wide
callback registry. Consumers no longer need to target native `ItemView` or `World`
for item-predicate policy. Register during startup, **before the first inventory
event**, with `register_item_predicate(PredicateRegistration { id, callback,
inventory_components })`. A true component flag explicitly requests the existing
inventory event adapter. Registration never occurs from predicate evaluation.
Repeated IDs are errors and do not replace their existing callback; a consumer
with multiple startup callers should use its own `Once` for idempotence.

The callback receives `PredicateRequest`: the unchanged JSON predicate, borrowed
`ItemSnapshot` fields, optional lossless `ComponentValue`, and a read-only
`TagLookup` interface. `serde_json` is a normal dependency of the existing shared
crate. No bundled-crate-to-bundled-crate or host dependency is introduced.
`ComponentValue` preserves numeric widths, arrays, float bit patterns and UTF-16,
including invalid surrogates. The library does not interpret component keys,
predicate discriminators, namespaces, ownership, eligibility or tag membership.
The native adapter alone converts host data and forwards tag queries.

Return `Unsupported` only if the callback does not own the predicate form. Return
`Handled(false)` for an owned form that is malformed or does not match. Callbacks
run in lexical registration-ID order, outside the registry lock. Dispatch uses a
snapshot: registration must finish before evaluation begins. One handler owns
the result. A second handler claiming ownership produces explicit `Conflict`
with the first two lexical owner IDs; the native adapter fails closed. Consumers
must not rely on callbacks later in the order executing after a conflict and
must not mutate state during evaluation. There is no runtime unregister/reset.

With no callbacks, the HEAD adapter returns `Continue` before metadata cloning
or conversion. If all callbacks return `Unsupported`, the original native matcher
runs unchanged. Otherwise a handled boolean is returned. Missing metadata outside
the existing inventory scope remains `None`; callbacks requiring it must fail
closed. The callback registry does not create a second metadata cache or registry.

PlutoLib owns one canonical `horizon_client_lib::advancements::item` boundary for
all callbacks. The two TConstruct criteria are separate consumer modules on the
acquisition recovery branch, with a startup registration hook. They are not part
of the central shared foundation and this does not certify migration of the
original acquisition PR stack. Tag expansion/indexing and other acquisition
host seams are still separate work.

## Synchronous custom event submission

`advancements::events` provides the host-independent `EventSink` capability and
borrowed `Event { trigger, payload, matches }`. The caller owns the event ID,
payload and event-specific JSON matching policy. Submit only after a real,
validated action succeeded. The shared library does not manufacture menu opens,
choose a block type, register criteria, grant IDs, or interpret payload keys.

The private native `pluto_advancement_events::with_game` adapter lends a sink for
the current host actor. Merely entering the callback submits nothing. A local
player uses local progress; inside the host's existing `as_guest` scope it uses
that authoritative guest, without choosing or remembering another actor. The
adapter rejects a remote client (`RemoteClient`) and an absent authoritative
actor (`NoActor`) before loading criteria or collecting player facts. It loads
the advancement tree before `player_entity_for(trigger)` so requested player
facts and NBT are available, then evaluates standard native player conditions
and uses the native trigger/progress/reward/notification path.

`Submitted` means only that the event reached that pipeline. It is not a receipt
for a matched/awarded criterion, persistence, network delivery or display.
Already-completed criteria remain governed by the native pipeline. The adapter
adds no automatic hooks, registry, actor cache, queue or delayed execution.
The sink and borrowed event must stay within the synchronous actor scope.

The TConstruct recovery branch demonstrates two existing successful controller
open boundaries (local and authoritative guest). Mapping controllers to source
types and matching the source trigger remain entirely in TConstruct. These
consumer changes are not part of the shared foundation. The bounded actor fixture
is not game/LAN proof; official focused type-check and full machine integration
are reported separately.

## Resolved item-tag overlays and inventory candidates

`advancements::tags` accepts resolved snapshots through a synchronous `TagSink`.
The producer owns tag resolution, namespaces, its managed-tag list, completeness
and readiness. `TagSnapshot` maps owned tag IDs to `Option<Vec<String>>`:
`None` is owned but unavailable; `Some([])` is explicitly known empty. `Lookup`
keeps `Unmanaged`, `Unavailable`, `Members` and defensive `Conflict` distinct.
Only unmanaged tags use native fallback. Unavailable/conflicting tags fail closed
in both native item-tag matching and candidate indexing.

`replace_owner` atomically replaces one owner's complete snapshot, normalizes
membership order/duplicates and rejects a tag already owned by another owner.
A rejection does not mutate either owner; consumers must handle that error.
An empty snapshot releases the owner. There is no namespace inference, resolver,
source-file parser, global provider registry or actor cache in this API.

The private native sink stores immutable snapshots and a separate custom index
cache on the current ChunkMap. The cache is an Arc-owned, type-erased payload
containing exact snapshot/tree identities and the computed index; only the game
adapter knows those host types. Wrong payload types or changed identities rebuild
rather than reusing the cache. The lower render layer imports only std types.
There is no back-edge dependency on the client library.

`replace_owner`, `clear` and `invalidate_candidates()` invalidate only this
custom cache. They never modify native `adv.inventory_index`, the inventory-event
hash, actor selection or progress. No-provider and released-provider paths use
the untouched native cache. Native tag/resource changes that affect an active
custom index require explicit custom invalidation if the tree/snapshot identity
did not change. Producers own ready publication at world/capture/reload boundaries.

Official Horizon parks/swaps worlds while retaining advancement state. Keeping
the custom cache on each world makes A-to-B-to-A transitions safe without an
AdvState field, process-global cache or unsafe address identity. Dropping a world
releases its cache and snapshots; no actor state is retained. Only ChunkMap's
supported explicit Default constructor is used. The earlier proposed AdvState
field was rejected by official planning and has been removed.

TConstruct's pure snapshot-publisher candidate retains its own acquisition-tag
allowlist and preserves unavailable/empty state. Runtime publication at source
capture, lifecycle readiness and removal of its old native tag/index hooks remain
step 3 work. Do not enable both implementations and assume cancellation order
provides composition. The shared foundation alone installs no TConstruct provider.

### Composition of copied native bodies

The component-event adapter conditionally replaces `Game::check_inventory_trigger`;
the managed-tag adapter conditionally replaces `Game::inventory_index`. Once an
adapter returns `Break`, another mod's hook inside the original body may never
run. A no-provider/default-off path does not make such a mixed installation safe.

`tools/check_advancement_plan.py` therefore requires the **complete installed
plan**, not a plan filtered to PlutoLib entries. Each of those two native methods
must have exactly the reviewed PlutoLib HEAD hook and no additional hook of any
kind. This deliberately conservative rule applies at the current priority 100;
it does not guess that a different priority makes an interior or return hook safe.
The guard also pins the official function fingerprint, source file hash, actor
binding and exact copied helper bytes. Any drift requires a reviewed guard update.

The central native preflight must call this guard before apply. A consumer
migration must remove its old replacements rather than rely on cancellation order.
Passing the guard proves the declared plan's composition contract, not that the
plan contains every mod the user later installs or that runtime behavior passed.
The official launcher does not automatically run this repository preflight.

The callback-based canonical item predicate is a different boundary: an
Unsupported result preserves native matching, and it is not a copied-body
replacement governed by this exclusivity rule. Its callback conflict and native
fallback contract remains unchanged.

## Explicit loaded-tree projection

`advancements::projection::TreeSink` lends read/replace access to the current
already-loaded authoritative tree. `get(id)` returns a normalized `Definition`
containing every field stored by the selected native host: parent, display,
criteria/conditions, requirements and all reward fields, including function.
It is not lossless original JSON; fields already ignored by the native parser,
whitespace and source formatting are unavailable.

A `Replacement` names an existing goal and supplies that complete expected
snapshot, new criteria, requirements and recipe-reward IDs. Parent, display,
experience, loot and function are retained automatically. The API does not
select source goals, validate a mod's goal shape, create goal IDs, add/remove
list entries or decide which recipe rewards a consumer should offer.

The complete batch is checked before publication. Duplicate goal IDs or criterion names,
dangling requirements, missing goals, stale expected snapshots, malformed native
index entries and definitions the native parser would normalize or drop fail
explicitly. Requests use the normalized native form, including qualified trigger
IDs and native criterion ordering. No partial tree is published after failure.
No-op requests preserve the original Arc. Changed requests preserve goal list
positions and the ID map, rebuild the trigger index and all native recipe,
looking-at, NBT and player-fact indexes, then swap the current tree once.

The private `pluto_advancement_projection::with_game` adapter rejects remote
clients and returns `NotLoaded` for the native empty-tree sentinel. It does not
load/join a prefetched tree or schedule a later replacement. Consumers own the
successful world-load/refresh timing. This capability changes a world definition,
not actor progress: it never grants rewards, resets the inventory-event hash,
modifies guest selection or rewrites earned progress. Both existing inventory
index paths compare tree Arc identity and rebuild on the next use; no new cache
owner or automatic hook is added. Consumers must preserve any source-specific
completed-goal behavior and recipe-revocation policy themselves.

Installing PlutoLib alone does not call this capability. Automatic source
projection, preload ordering and old consumer hook removal remain separate
consumer migration work. See the focused projection validation report for the
exact behavior proof and its limits.
