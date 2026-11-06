# Storage boundaries for feature-group reorganization

Read-only inventory, 2026-11-06. This is classification input, not a PR reorganization or implementation change. The reviewed baseline is [published v15](https://github.com/sofia-gros/horizon-mods/commit/89744814b8d536a117e82f8ea3c8cdc9798faa95) (local `8715b123`), with official Horizon `624c4f62` / Mixins `e8cd0aa3`. Its ordinary Pluto package has plan/apply/host lib+bin type evidence. That is not complete TConstruct, gameplay, save/reload or LAN evidence.

## Storage contract layers

The coordinator’s provisional eight feature categories are: ① Pluto foundation, ② items/material/data, ③ recipe/station/advancement, ④ tools/combat, ⑤ armor/movement, ⑥ machines/storage, ⑦ world-fluid/render, ⑧ guide/UI/LAN. Final parent/user approval is pending. This inventory primarily serves ⑥ and shared ①; equipment authority crosses ④/⑤, and display mapping touches ⑧ without claiming its transport lifecycle.

Within that provisional taxonomy, keep storage/inventory as a coherent feature group with explicit contract layers, rather than one PR per helper:

1. **Pure proposals:** movement planning, candidate/stale guards and partitioned copies.
2. **Native representation:** item codec eligibility/canonicality, two distinct snapshot contracts and owned NBT patches.
3. **Plain inventory values:** cell operations, explicit ordered insertion and read-only projections.
4. **Consumer integration:** TConstruct schemas, admissions, layout, physical indexes and publication/notification order. The two verified quantity follow-ups belong here.

Machine publication and equipment wear authority are explicit unresolved boundaries, not implementations hidden in a generic storage wrapper. A classification row marked implemented refers only to its stated library contract.

## Existing use and remaining boundaries

### Move Planning

Borrowed logical source/destination, count/acceptance/limit/equality callbacks; no mutation.

- Library: Implemented and packaged in v15.

- Actual consumer: Chute/casting/Cannon hopper planning and Chest insertion reuse planner; exact later Chest quantity change is a separate follow-up.

- Keep local: Physical layout, acceptance, per-slot limit, candidate order, callback and commit.

- Remaining gate: Preserve policy and prove each migrated caller separately; not a universal menu transaction.

- Sources: [planner](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/lib.rs), [hopper](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-chute-hopper.rs), [chests_base](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-tinker-chests.rs), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-v15-candidate-20261106.json).

### Candidate And Stale Guards

Ordered select/continue/abort; present snapshot equality; encode/decode guard with caller-supplied identity.

- Library: Implemented and packaged in v15.

- Actual consumer: Hopper callbacks use candidate selection plus live inventory and raw-record checks.

- Keep local: Missing chunks/blocks, raw storage identity, native callback effects and rollback.

- Remaining gate: Full live caller composition remains outside value-helper proof.

- Sources: [planner](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/lib.rs), [hopper](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-chute-hopper.rs).

### Saved Item Codec

Actual host NBT roundtrip eligibility and native Tag canonical decode; reject zero count and recursive retained wire for saving.

- Library: Implemented and packaged in v15.

- Actual consumer: Casting, Heater, Cannon, Casting Tank, Smeltery serializer and canonical checks delegate.

- Keep local: Raw-slot empty/legacy forms, aliases, fields, count splitting, existing-record canonicality and recipe progress.

- Remaining gate: Do not substitute canonical Tag equality or save eligibility for a live snapshot/transport guarantee.

- Sources: [codec](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/stack-storage.rs), [fuel](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-smeltery-fuel.rs), [casting_identity](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-casting-tank.rs), [melter_state](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-melter-state.rs), [smeltery_state](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-smeltery-state.rs).

### Observable Snapshot

Count/components plus recursive retained/usable wire bytes and runtime BundleSelected.

- Library: Implemented and packaged in v15.

- Actual consumer: Hopper and held Melter guards use observable comparator.

- Keep local: Actor/world/physical-slot scope and authoritative phase.

- Remaining gate: Private decoded Wire baseline remains excluded; no arbitrary live write authority.

- Sources: [codec](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/stack-storage.rs), [hopper](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-chute-hopper.rs), [machine_use](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-machine-use.rs).

### Private Retained Baseline

Read-only exact baseline graph comparison, native item_data child read seam, completed-pair memoization only.

- Library: Additive implementation; applied-items 8-case proof and package gate recorded.

- Actual consumer: No automatic comparator replacement or consumer adoption.

- Keep local: Consumer acceptance decision, lifecycle and write authorization remain separate.

- Remaining gate: Any adoption needs its own scope/behavior proof; pointer equality is not value equality.

- Sources: [private_snapshot](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/stack-snapshot.rs), [identity_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/item-private-baseline-validation-20261105.md), [equipment_audit](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/equipment-authority-audit-20261105.md).

### Owned Compound Patching

Clone untouched JavaString/NBT values; ordered set/delete, last update wins; noncompound refusal.

- Library: Implemented and packaged in v15.

- Actual consumer: Tank/Faucet/Melter/Casting/Smeltery owned fields use shared patching.

- Keep local: Exact owned-key lists, aliases, unknown fields, optional/empty policy and nested schema.

- Remaining gate: No inferred recursive merge, persistence, notification or atomic multi-record commit.

- Sources: [nbt](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/nbt-storage.rs), [faucet](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-faucet-tank.rs), [melter_state](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-melter-state.rs), [smeltery_state](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-smeltery-state.rs).

### Partition Staging

One independent proposal copy per key; input-order edits; sorted-key successful proposals; failure drops proposals.

- Library: Implemented and packaged in v15.

- Actual consumer: write_many stages chunk metadata before caller-owned publication.

- Keep local: set_metadata whitelist/ID policy, exclusive World publication, dirty marking and lifecycle.

- Remaining gate: Callbacks must not mutate live aliases; external effects cannot be rolled back by dropping copies.

- Sources: [staging](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/storage.rs), [store](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-melter-store.rs).

### Plain Cell Operations

pickup/take/collect_ordered over explicitly borrowed cells, supplied equality and limits.

- Library: Implemented and packaged in v15.

- Actual consumer: Chest/player wrappers delegate; dynamic admission and existing callers retained.

- Keep local: Zero-count/zero-limit fail-closed correction is explicit in original cell migration; physical mappings, visibility, Part admission, native menu hooks remain local.

- Remaining gate: Keep original correction separate from the later exact-parity insert_slot follow-up.

- Sources: [cells](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/stack-cells.rs), [chests_base](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-tinker-chests.rs).

### Ordered Insertion

Independent merge/empty orders with caller equality and limits, quantities from plan_between.

- Library: Additive API implemented, 39-test focused proof, included in v15.

- Actual consumer: source_give_inventory and source_quick_move_player have not been replaced with this API.

- Keep local: Old give can insert Some(count=0) and return None; API rejects zero before mutation. Builder pattern gates, short-input indexing and result side/player order are distinct.

- Remaining gate: Not a drop-in migration. No normalization or another arithmetic API proposed from this audit.

- Sources: [cells](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/stack-cells.rs), [builder](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/transaction.rs), [ordered_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/ordered-insertion-validation-20261106.json), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-v15-candidate-20261106.json).

### Display Projection

Explicit mapping/order, duplicated indexes, empty/padding cloning; read only.

- Library: Implemented and packaged in v15.

- Actual consumer: Chest main/side and Smeltery page projections delegate.

- Keep local: 54-slot transport, 32/48 real cells, scrolling, reserved fluid cells, inbound mapping and stale-page/no-wire gates.

- Remaining gate: No transport authority or current-state/write guarantee follows from projected values.

- Sources: [projection](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/projection.rs), [chests_base](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-tinker-chests.rs), [smeltery_page](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-smeltery-page.rs), [projection_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/inventory-projection-validation-20261105.md).

### Slot Quantity Followup

Existing insert_slot now calls plan_between on two snapshots instead of whole-inventory temporary snapshot.

- Library: No new API.

- Actual consumer: Verified independently and published; not part of the frozen v15 proof baseline.

- Keep local: Whole-inventory first_equal, Part admission, physical index, mutation/remainder order and all malformed behavior retained.

- Remaining gate: Select as coherent storage consumer follow-up; 42 tests/9019 new comparisons do not prove full TConstruct or live LAN.

- Sources: [slot_followup](https://github.com/sofia-gros/horizon-mods/blob/375fa4aef1e70f17e5d79d2e5bbf94bbcb87192f/tconstruct/mixins/tconstruct/source-tinker-chests.rs), [slot_proof](https://github.com/sofia-gros/horizon-mods/blob/375fa4aef1e70f17e5d79d2e5bbf94bbcb87192f/pluto-lib/docs/slot-insertion-validation-20261106.json).

### Fluid Quantity Followup

Existing click/quick-input incoming quantity delegates to plan_between.

- Library: No new API.

- Actual consumer: Verified independently and published; not part of the frozen v15 proof baseline.

- Keep local: Empty-cursor branch unchanged; known_fluid evaluation order/count, output gate, exact equality, nonswap policy,1/16/64 limits and mutation order retained.

- Remaining gate: Select separately from prior slot follow-up; 144 executions include139 existing tests, with23220 new differential cases; no live Game/save/LAN claim.

- Sources: [fluid_followup](https://github.com/sofia-gros/horizon-mods/blob/b0e911b5a294c430a471e168f86c7ca0ed7152f4/tconstruct/mixins/tconstruct/source-fluid-container.rs), [fluid_proof](https://github.com/sofia-gros/horizon-mods/blob/b0e911b5a294c430a471e168f86c7ca0ed7152f4/pluto-lib/docs/fluid-slot-quantity-validation-20261106.json).

### Machine Publication

No shared commit service established; representation/staging helpers already cover mechanical preparation.

- Library: Shared write/durable-save/sync/rollback API is NOT implemented.

- Actual consumer: TConstruct owns metadata -> live slots -> comparator/container notifications -> held/inventory publication.

- Keep local: Schema whitelist, casting vs Cannon raw identities, creative handling, native neighbor/spill effects, client write_snapshot replacement semantics.

- Remaining gate: Need actual native exchange/phase/actor assertions before changing sequence. write/write_many only modify in-memory saved sources and dirty flags, not fsync/durable storage.

- Sources: [store](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-melter-store.rs), [machine_runtime](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-machine-runtime.rs), [machine_audit](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/machine-commit-phase-audit-20261105.md), [casting_identity](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-casting-tank.rs).

### Equipment Wear Authority

Value/read facts are available, but no physical-slot/actor/world invocation write capability exists.

- Library: Generic mutation boundary is NOT established.

- Actual consumer: Active retained-tool paths keep TConstruct durability_loss bypass policy; dormant armor end/record remains disabled.

- Keep local: Break retention, overslime/modifier causes, RNG, native damage hook/advancement choices, feedback and commit order.

- Remaining gate: WRAP target plan rejected unsupported control flow; native_item_damaged RETURN diagnostic only planned successfully, not installed/applied/runtime verified. Neither establishes mutation authority.

- Sources: [equipment_audit](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/equipment-authority-audit-20261105.md), [ranged_audit](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/ranged-wear-phase-audit-20261105.md), [wear_probe](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/native-wear-phase-plan-20261105.md).

## Reorganization cautions

- Save eligibility, canonical saved Tag identity, observable live snapshot identity and retained private-baseline identity are four different questions. None binds actor/world/physical slot. Do not consolidate them into a single “lossless transaction” status.

- `insert_ordered` is implemented but intentionally not a drop-in replacement for the old zero-count behavior. The existing Part uniqueness, pattern and side/player routing rules are consumer policy, not missing generic arithmetic.

- `write_snapshot` replaces received data rather than applying server preservation overlays. `write` / `write_many` modify in-memory sources and mark chunks dirty; they do not establish durable disk persistence, network delivery or rollback across native notifications.

- Machine notification can run neighbor work and spill processing before a later held-slot update. Equipment native wear can consume RNG, hooks and advancement effects before item mutation. A comparator cannot undo those effects or authorize their reordering.

- The slot follow-up is published at [375fa4ae](https://github.com/sofia-gros/horizon-mods/commit/375fa4aef1e70f17e5d79d2e5bbf94bbcb87192f), and the fluid follow-up at [b0e911b5](https://github.com/sofia-gros/horizon-mods/commit/b0e911b5a294c430a471e168f86c7ca0ed7152f4). They are separate overlays on the recorded foundation, not a combined runtime proof.

- No production/manifest/PR changes and no new native tests were performed for this report. Existing test counts remain scoped to their exact receipts; repeated module executions are not summed as unique scenarios.

The [machine-readable inventory](storage-reorganization-inventory-20261106.json) records full source revisions, content hashes and checked symbol anchors for each classification.
