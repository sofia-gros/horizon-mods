# Ordered tag-graph resolution

The shared native module `horizon_client::pluto_tag_graph` expands a normalized
tag graph into an ordered set of registered value IDs. It contains no Tinkers
IDs, JSON schema, namespace defaults, pack ordering or Forge-specific parser.

Each entry names a value or another group, with an add or remove operation.
Adds can be required or optional. The caller supplies a stable group lookup,
registry membership query, and an explicit missing-value policy:

- `Reject`: missing required values fail the group.
- `Ignore`: project onto the available registry, omitting missing values.

First insertion determines order. A removal followed by an add moves the value
to the end. Missing or invalid optional/removal groups are skipped entirely,
including their partial results. Cycles remain typed errors even on optional
or removal edges. Missing and invalid root groups are distinct errors.

An explicit traversal stack replaces native recursion. The implementation does
not impose a resource budget or cache repeated expansions. Callers are still
responsible for the size and stability of the input snapshot.

## Structural reachability

`reaches_value(root, value, edges)` searches a caller-selected graph without
consulting a registry or interpreting tag operations. The edge callback returns
plain `Target::Value` and `Target::Group` values. An empty edge list has no
reachable values; the caller owns any missing/invalid-group interpretation.

The search is iterative and depth-first in supplied order. It returns `true`
immediately when it finds the value, or `false` after exhausting the graph.
An active-path cycle encountered before a match returns `Error::Cycle`; branches
after a match are not inspected. Repeated groups on completed paths are allowed.
There is no global cache, registry lookup or positive membership claim. A
reachable raw value may still be invalid or removed during full resolution.

## TConstruct policy boundary

`source-tags.rs` retains raw file parsing, invalid-file handling, pack replace
and remove interpretation, default `minecraft` namespace, Mantle water and mud
compatibility bridges, registry completeness and canonical modifier identity.
Its item/modifier resolver chooses `Reject`; its available-block resolver
chooses `Ignore`. Existing error labels and public snapshot shapes remain the
same. The conservative negative-membership shortcut also remains
TConstruct-owned, but delegates its traversal to `reaches_value`. TConstruct
filters removal edges, keeps required and optional additions as possibilities,
canonicalizes the source namespace, and maps missing or ignored-invalid files
to empty edge lists. `local_tag_member` continues to reject missing or wholly
malformed roots before considering a negative result. Source callers' policies
and the distinction between possible and resolved membership are unchanged.

Standalone fixtures import the real graph source under the native library's
module name. No second resolver implementation is copied into test doubles.

## Combined integration

The central integration owner adds one public module to the unified manifest:

```toml
[[module]]
id = "tag-graph"
file = "crates/horizon-client/src/client_lib.rs"
name = "pluto_tag_graph"
source = "tag-graph.rs"
visibility = "pub"
```

This library export is intentional. TConstruct uses the resolver in both the
game executable and the reusable client library. `pluto-api` is currently only
a dependency of the executable, so placing this engine solely in that crate
would break the library consumer. No duplicate Cargo package or host Cargo
edit is needed. The native facade may re-export this same module.

This service and raw resource layers are one shared-resource feature batch.
The integration owner manages the facade, manifest and packaged host proof.

## Focused evidence collected before batch integration

Eight graph tests passed, including nested failure unwinding and a 3,000-group
chain. Nineteen existing source-tag tests and fourteen modifier-snapshot tests
passed using the production wrappers. An earlier temporary public-API export
also passed the eight graph tests, before the final library-export routing was
selected. The final routing and combined official package still need central
validation; those earlier results are not a host-build claim.

The later reachability slice adds four shared traversal tests (early match,
cycle ordering, repeated completed groups and a 3,000-group chain) and two
TConstruct policy regressions. They are written but not individually executed;
review and tests are deferred to the requested combined feature batch. The
existing public module and fixture imports cover the new function, so this
slice adds no manifest, dependency, state or host injection.
