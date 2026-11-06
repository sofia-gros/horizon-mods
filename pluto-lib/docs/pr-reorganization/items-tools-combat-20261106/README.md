# Remaining items, tools and combat PR intake audit

Read-only regrouping evidence for 37 fixed PR heads. No PR, runtime code, native plan, compiler, cache or production state changed. The proposed primary groups below are review ownership, not approved PR regrouping or migration completion.

## Architecture and cutoff

Horizon provides the engine, Mixins the supported extension layer. Mods opting into PlutoLib use its shared services above Mixins; other Mixins mods may use Mixins directly. TConstruct uses PlutoLib. This does not move TConstruct IDs, recipes, formulas, layouts or actor policy into the shared layer.

Fixed main: `08afa992d5091e03360f3473ea93fd17aa793987`. Central local `8715b1232c0fca133fe012f35830e2108e653d59` equals remote v15 `89744814b8d536a117e82f8ea3c8cdc9798faa95` by tree. The 265-PR graph is frozen in [4bf5fe7d](https://github.com/sofia-gros/horizon-mods/blob/4bf5fe7d06a08ae72330dea44ddfc445471f073a/pluto-lib/docs/pr-reorganization-snapshot-20261106.json).

## Findings

- 37 selected PRs, 21 stacked, 346 own file-delta records across 223 paths. Their 47 commits in own merge-base ranges are distinct.
- None of these 37 heads is an ancestor of fixed main or central. None of the 346 new file states exactly matches the corresponding central file state. This is not a claim that every behavior is absent or that independently evolved equivalents do not exist.
- #676 and #702 have REST-reported bases that are not head ancestors. Their own deltas use actual merge bases, not a reverse endpoint comparison.
- #683 and #691 are merge tips. A first-parent cherry-pick is not equivalent to their actual reported-base merge-base ranges.
- There are no identical whole-diff stable patch IDs or identical same-path old/new blob transitions within these 37. This does not rule out semantic overlap, copied subexpressions, later overrides or overlaps with other groups.
- 52 same-path intersections require review; 22 cross this selected scope. Same path does not mean same feature or proven conflict.
- Three exact native target intersections with Pluto occur at ChunkMap::clear (#806, #862, #928). Actual source/composition guards, ordering and authority still determine safe coexistence.
- 13 heads contain lexical Pluto references in changed Rust files, mostly inherited stack-fluid/creative helpers or test includes. They do not establish that the new feature was migrated to the current v15 contracts.
- #936 has six own changed paths versus 29 cumulative paths from its main merge base. Selecting only the last title or replaying the entire terminal tree would confuse inherited feature content.

## Contract intake

### canonical data and compatibility

PRs: #691, #699. Proposed primary review group: 02-items-material-data.

Generic raw resource/tag traversal and the existing creative registration interface are available.

Keep in TConstruct: Canonical namespace/removal rules, modifier eligibility and the ownership fix for scute test compatibility remain TConstruct-specific.

Remaining: Current shared tag traversal does not establish that the complete original modifier change or test harness has been migrated.

### native loot and equipment facts

PRs: #676, #796, #799, #802, #806, #808. Proposed primary review group: 04-tools-combat.

Equipment snapshots, resource/tag queries and native item representation are reusable read facts.

Keep in TConstruct: Necrotic Bone rolls, Fortune/Looting levels, crop/gold eligibility and launch-to-death feature policy.

Remaining: Mutable death-loot context and projectile identity/lifetime are not granted by those read facts. Historical #799/#806 mutable-killer finding is a source finding, not a new compile result.

### held tools ammunition and wear

PRs: #559, #683, #702. Proposed primary review group: 04-tools-combat.

Existing quantity planners and item snapshots can support explicit operations without owning the Game tail.

Keep in TConstruct: Momentum, Multishot, Expanded formulas, charge/ammo rules, area eligibility and retained-tool break policy.

Remaining: Shot entry is not the later wear invocation. Native RNG/hooks/advancement, selected physical hand and actor scope need their own contract; native wear is not equivalent to the custom durability-loss bypass.

### authoritative inventory tick

PRs: #706, #715. Proposed primary review group: 04-tools-combat.

Existing stack-fluid value operations and storage representation helpers are available.

Keep in TConstruct: Overgrowth/Overburn eligibility, fuel rules and Overslime refill amounts.

Remaining: Original TAIL declarations are retained historical blockers. Generic quantity helpers do not supply an authoritative tick completion or live actor lifetime.

### primary hit and projectile context

PRs: #734, #772, #777, #783, #876, #877, #878, #879, #880, #881, #882, #883, #884, #885, #886, #928, #936. Proposed primary review group: 04-tools-combat.

The v15 foundation supplies generic equipment/read-only damage facts, not a general accepted-hit or projectile-mutation authority.

Keep in TConstruct: Padded, Insatiable, Spectral, Smite, Antiaquatic, Pierce, Bane, Enderference, Antitoxin, Flamestance, Lacerating, Scorching, Conducting and offhand formulas/selection.

Remaining: Old attack_mob interior rewrite restrictions and phase/order/accepted-hit state must be resolved per real caller. Exact ChunkMap::clear target intersections require full composition review; matching strings alone neither approve nor prohibit composition.

### dropped item lifecycle

PRs: #817, #829, #833, #837. Proposed primary review group: 04-tools-combat.

No existing value/transaction helper certifies dropped-item expiry, explosion or lightning lifecycle authority.

Keep in TConstruct: Worldbound acquisition and protection eligibility, source item metadata and environmental policy.

Remaining: Historical TAIL blockers and native lifetime/timing need a supported actual owner boundary; do not simply enqueue effects or delete old hooks.

### held teleport

PRs: #854, #862. Proposed primary review group: 04-tools-combat.

Read-only current values do not confer a timed-use or authoritative guest teleport capability.

Keep in TConstruct: Warping acquisition, charge/duration, held-item eligibility and destination selection.

Remaining: Preserve local/guest call order and actual world/actor owner around teleport. Native clear-hook composition is separate from successful teleport execution.

### equipment curative lifetime

PRs: #825. Proposed primary review group: 05-armor-defense-movement.

Pure curative policy values and equipment read facts exist. Unapplied instance/NBT/contact proposals are research only.

Keep in TConstruct: Strong Bones milk Resistance, helmet eligibility and curing policy.

Remaining: Actual per-instance hidden chain/save and synchronous native actor authority are not implemented by a shared value policy or by unexecuted Game borrow probes.

## Evidence and reproduction

`ledger.json` contains immutable refs, actual merge bases, own commits, cumulative comparisons, descriptor changes, target intersections and lexical read results. `file-deltas.json` preserves full old/new Git blobs and modes, no-renames status and exact per-file patch SHA256. `intake-notes.json` separates current source anchors from unchanged historical findings.

Run the read-only auditor with an existing MOD checkout and the exact published snapshot:

    python reproduce.py --repo /path/to/horizon-mods --snapshot pr-reorganization-snapshot-20261106.json --out /tmp/remaining-pr-audit

It performs no fetch, checkout, compiler run or PR write. The two external snapshot heads #944/#945 are not present as local Git objects, so their external path comparison is explicitly omitted here; the Guide ledger resolves their remote ancestry separately. All 37 selected head/base objects are present.

The reproduced JSON outputs must byte-match the frozen output. This audit does not rerun historical official-planner diagnostics or promote them to native execution evidence.

## Integration limits and priority

Keep source-owned recipes/modifier selection/formulas and all meaningful ordering. Available snapshots, quantity planners, pure curative values and read-only observers do not supply mutable Game ownership, save durability, rollback or transport authority. No wrapper is unconditionally safe to remove from this audit.

Step 2 should prioritize coherent adoption of already-proved services and the real missing owner/phase boundaries, rather than accumulating trivial helper APIs. The independently proved Java/menu/slot/fluid deltas are separate from the frozen v15 composition. A future combined gate is required after integration.

Actions are currently unavailable to the user. Native CI and Game/renderer checks remain unrun. This is not a reason to attempt an unsafe local compiler replay under an unknown RAM limit. Source-only integration planning and exact-diff preservation can continue.
