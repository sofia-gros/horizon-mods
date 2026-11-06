# Shared APIs and host adapters

PlutoLib provides reusable APIs for multiple mods. It owns the connection to
official Horizon and Mixins. A consuming mod owns its content, recipes, policy
and formulas. TConstruct is one consumer, not a dependency of PlutoLib.

## The dependency boundary

Keep the public operation and its host adapter separate:

- Generic Rust code accepts values, snapshots and callbacks. It does not import
  a consuming mod, assume its namespace, or interpret its private component keys.
- Native adapters obtain authoritative host state, translate it to the public
  contract, invoke registered policy at the documented phase, and apply results
  under the host's permissions and lifecycle.
- Consumers decide eligibility, capacities, recipe semantics, material effects
  and presentation content. Moving these decisions into the library would make
  other consumers inherit TConstruct's rules.
- Registration and scoped data have one owner. A consumer must not copy the
  registry module into its own crate or introduce a second static instance.

The current native package contains two generic crates. `pluto-api` is attached
to the gameplay package; `pluto-inventory-api` is attached to the shared client
library. The latter is re-exported once as `horizon_client::pluto_inventory` for
gameplay adapters. Predicate helpers inside the shared client library can use
`pluto_inventory_api` directly. These paths do not create duplicate registries.

This structure follows the current official Mixins dependency rules: a bundled
crate has one dependent host package and may depend only on registry packages
already present in the host lockfile. Sibling bundled/path/git dependencies and
reverse host dependency cycles are not supported. A single facade must not be
simulated by copying source or state into every layer. Host-qualified adapter
paths are documented explicitly; they are not a universal cross-host ABI.

## Current foundation contracts

| Service | Library responsibility | Consumer responsibility |
|---|---|---|
| Inventory planning | Pure move proposals and snapshot/count information | Slot acceptance, effective limits and authoritative commit |
| Transaction guards | Candidate ordering, stale-snapshot rejection and encode/decode identity checks | Machine ordering, permissions, recipes and rollback policy |
| Native stack storage | Observable count/component/wire-state comparison (not private Wire baseline identity), lossless host codec boundary and staged compound field patches | Owned schema fields, aliases, migrations, empty-slot semantics and which state must be saved |
| Advancement services | Synchronous metadata scope, component-aware change keys, stable predicate callbacks and explicit event submission | Predicate meaning, trigger IDs, acquisition rules, rewards and startup activation |
| Resource and tag mechanics | Validated resource traversal, explicit unreadable-file policy, ordered raw layers and generic graph expansion/reachability | Namespaces, selected resource categories, tag removals, redirects and recipe/material semantics |
| Protection | Scoped native damage-stage plumbing, provider dispatch and generic signed/capped protection arithmetic | Equipment policy, modifier formulas, eligibility and provider registration |
| Fluid services | Definitions, atomic registry snapshots, bounded contact queries, source transactions, finite flow and built-in host facts | Content assets, fluid families, capacities and interaction/effect rules |
| Recipe-book services | Explicit UI invalidation, read-only checked ghost previews, generic preview cells and one-shot saved-knowledge handoff | Recipe IDs, allocation strategy, component eligibility, restoration readiness and placement decisions |
| Guide documents | Bounded portable document codec and restricted native Text conversion | Content, title/layout, metadata admission, window/session ownership and transport selection |
| Rendering and catalog facts | Exact tinted crop/tile/cuboid mechanics, light packing and read-only potion/effect records | Sprites, color formulas, geometry/layout, effect application and actor attribution |
| Creative/item APIs | Generic registration and documented short-use dispatch | Content definitions and item behavior |
| Existing tool/fluid adapters | Native access and generic operations documented by each adapter | Tool restrictions, fluid choice, capacity and effect rules |

See [inventory](inventory.md), [plain inventory cell operations](plain-stack-cells.md),
[advancements](advancements.md),
[tool rules](tool-rules.md), [item use](item-use-b2.md) and
[component transport](component-transport.md) for exact limits. In particular,
snapshot equality and save eligibility are different operations: retained wire
data can be relevant to identity even when a particular codec cannot save it
losslessly.

This foundation is not the completed migration of all active TConstruct PRs.
The table records extracted responsibilities, not a claim that every original
consumer now uses them. The complete library package is now checked as one composition; see the
[immutable integration report](shared-foundation-validation-20261105.md).
Consumer activation, live damage/equipment behavior, guide sessions, persistent
recipe-index ownership, component-safe placement and the full custom-fluid
rendering/effect surface still need their own evidence. Existing feature
branches are not silently imported merely to make foundation tests build.

Broad feature groups share a common official pin and packaging base. Keep each
service together with its consumer migration and tests; do not create a separate
PR for every helper or affected stacked head. A complete library package check
proves composition of the installed library adapters, while a focused consumer
fixture proves only its named boundary. Neither is a full TConstruct gameplay,
save/reload or multiplayer result.

For a feature-group view of extracted and remaining work, see
[the migration ledger](shared-api-migration-status.md).

## Adapting to a new official target

PlutoLib tracks official Horizon and Mixins at an immutable pair for each
integration candidate. Update private adapters inside PlutoLib first. Preserve
native control flow, callback ordering, permission checks, error behavior and
the no-consumer path. A copied native body needs attribution and a drift check;
matching the function name is not enough.

Do not suppress a missing host capability or move an operation to a different
phase just to make the compiler accept it. Record the gap, keep consumer policy
separate and prove any alternative at the original semantic boundary. When the
old contract cannot be maintained, publish an explicit API change rather than
claiming unconditional compatibility.

The [official recovery baseline](official-upgrade-20261105.md) distinguishes
source inspection, pure tests, packaged apply, host checking/building and
playable/save/LAN validation. A focused proof covers only its named source and
target. It does not validate a whole consumer branch or every native component.
