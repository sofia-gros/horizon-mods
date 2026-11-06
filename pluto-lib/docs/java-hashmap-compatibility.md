# Opt-in Java linked-bin ordering compatibility

## Scope and real callers

The shared `pluto_inventory_api::java_hashmap` module supplies two pure operations:

- `utf16_hash` computes Java String hash bits over caller-provided UTF-16 units.
  It preserves lone surrogates and uses wrapping arithmetic; it does not normalize.
- `linked_iteration_order` maps raw key hashes, in insertion order, to traversal
  indices using the default Java HashMap capacity/load factor and linked bins.
  It rejects a transition to a tree bin rather than inventing an order.

This extracts the algorithm already used by TConstruct's `recipe-order.rs`.
The actual call chain is `source-builder-world.rs::lookup_material` →
`transaction.rs::source_material_lookup` → `java_recipe_order`. Builder preview
and material planning paths and `source-station-world.rs` repair selection call
that lookup. The old file's claim that this had no gameplay caller was stale.
This source trace is not a claim that those whole consumers are validated or
activated against the latest official host.

Other Java-content ports can use the same linked-bin mechanics with their own
key hashes and selection rules. No TConstruct IDs, recipe fields, Forge metadata
rules, item eligibility, native hooks, registries or state live in this module.
The compatibility contract is intentionally narrower than a complete Java map.

## Caller-owned decisions

The caller must pass one entry per distinct logical key, after accounting for
key equality and replacements. Equal hashes do **not** mean equal keys. The API
cannot deduplicate hashes or validate key identity. All keys are considered new
insertions; removal, reinsertion, custom initial capacities/load factors and
Java tree-bin behavior are outside this contract.

TConstruct continues to validate its existing namespace/path shape, compose the
ResourceLocation hash, sort by path then namespace, deduplicate equal IDs, and
require the complete parsed recipe set including metadata. It maps an unsupported
tree bin to its existing `UnsupportedOrder` outcome and retains all conditions,
recipe-family and material selection. Validation was not broadened or tightened.
No fallback sorting is added.

Hash values are the raw 32-bit Java `hashCode` representation. The library applies
the xor spread itself. At the ninth collision it resizes below capacity 64; at
capacity 64 or above it refuses treeification. Resizing preserves low/high linked
subsequence order, then traversal proceeds by ascending bucket number.

## Official-host boundary

Official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` uses
`crafting::registration_key` and `Recipes::from_sources` for its native recipe
order. That is a different contract. This service is opt-in compatibility for
an existing older source-data consumer, **not** a native ordering replacement.
No official source or native manifest changes are required. The existing shared
inventory API package has one owner and its ordinary recursive packager includes
this new source module. Complete-package validation remains a later separate gate.

## Validation design and current status

The committed fixture links the real inventory API Cargo package and includes
both the migrated consumer file and a byte-exact frozen original. It compares
1,285 ordinary input presentations (lengths 0–256, reordered and duplicated) plus
invalid-ID, Unicode and collision cases. Separate tests exercise raw hash spread,
load resize, repeated hash identity, tree refusal and UTF-16 wrapping. Both old
and migrated source modules also retain the existing Java-generated golden vector
and deliberate tree-bin rejection test.

The existing full pinned Minecraft/Forge/TConstruct Java comparison tool now runs
the actual Cargo consumer with `--locked --offline`, rather than compiling a
standalone source file without the real API dependency. It still requires its
pinned input archives, the source checkout and installed Java tools. Those inputs
are not replaced with a synthetic corpus and that full comparison is not claimed
as executed by the focused fixture.

## Focused result

Exact runtime `18976165d9623057730464c277a55a22c6af53fd`, with test-only
path correction `a92a41b5fcfff52373d6150f7221d2cb683bbd45`, passed eight Rust
tests and seven Python guards. The Rust fixture performs 1,311 legacy result
comparisons and retains both the old and migrated Java-generated golden/tree
cases. It also checks raw-hash spread, resize partition order, equal-hash distinct
entries, tree refusal, UTF-16 surrogates and wrapping. Four deliberately corrupted
consumer policies are rejected by the source guard.

The first exact runtime attempt failed before tests because the fixture path had
one extra `../`. Its log is preserved. The correction changes only test wiring
and adds an exact-path guard; no runtime code or expected result was weakened.
All 41 captured API/fixture/lock/source hashes matched before and after the
successful run. Locked registry package identities match the pinned official
host lock. The continuous disk floor held (minimum 10,883,473,408 bytes).

No live Java comparison ran: `javac` was unavailable on PATH. The golden vector
is historical independent Java evidence; frozen legacy equivalence is a separate,
non-independent regression check. The full pinned input corpus, full API unit
suite, native package/apply/build and gameplay were not run. The focused fixture
compiles the real API crate but does not claim all its operations were exercised.
The [receipt](java-hashmap-validation-20261106.json) records the exact cutoff,
initial failure, successful proof and source hashes.
