# Read-only potion and effect facts

PlutoLib owns the native registry boundary through the public client-library
module `horizon_client::pluto_potions`. This reusable service has no TConstruct
content dependency and does not register effects, select recipes, inspect active
player effects, or apply/clear effects. No injection, state or new crate is added.

- `effect(id) -> Option<EffectFacts>` preserves the official lookup aliases and
  returns the raw storage ID, canonical mob-storage ID, unmixed RGB color and
  instantaneous flag. Vanilla raw IDs are short; canonical IDs include
  `minecraft:`. Registered custom IDs retain their namespace in both fields.
- `potion(id) -> Option<PotionFacts>` exposes the raw potion ID, translation-name
  suffix, and ordered `(raw effect ID, duration ticks, amplifier)` slice. Empty
  known potions remain `Some`; missing definitions remain `None`. No filtering,
  sorting, deduplication, source-version allowlist or size cap is introduced.
- `has_custom_namespace(id)` preserves the host's **syntax-only** classification.
  It does not validate the identifier or establish registration. For example,
  `not_registered:missing` can return true while `effect` returns None.

These are Pluto-owned Rust structs and primitive values, not native definition
re-exports. The adapter uses the official process-lifetime catalog and its
canonical-name memoization; no separate Pluto cache or resource snapshot is
created. A host update that changes catalog ownership or identity semantics must
be handled here and revalidated. This is not an API for a source mod's registry.

## Consumer boundary

Six TConstruct helpers adopt the facts: Melter tint, potion display names,
fluid-effect parsing, Slurping, Spilling and Splashing. TConstruct retains its
source-version potion allowlist, NBT schema and 64-custom-effect cap, float32
weighted color and alpha packing, effect ordering/duration/scaling, immunity and
cure rules, actor selection, transactions and all effect application. Its
existing cloud-color call to the host's integer average is outside this catalog
extraction and unchanged.

`tests/potion-catalog-consumer-sources.json` records the six exact originals at
`d6fba7eb` and normalized full-file policy hashes. Normalization permits only the
catalog function paths, equivalent fact field names, canonical-ID projection and
syntax-only predicate rename. Every other consumer byte must remain identical.
The source fixture for fluid selection imports the actual adapter, not a mock.
Storage fixtures require the same actual module in their shared root include;
that wiring is coordinated with the storage group rather than duplicating its
newer fixture definitions on this branch.

## Official source and validation

Target: official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, with Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02` for package composition. Native catalog
semantics are in [official potions.rs](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client-core/src/potions.rs):
`effect` at lines 114–120, syntax classification at 124–126, canonical IDs at
162–179, `PotionDef` at 183–187, and potion lookup at 292–295.

Four boundary checks, fourteen rendering/identity regression checks and nine
selected existing consumer checks pass (27 total). An initial consumer source run could
not read two workflow files omitted by the sparse checkout; after adding only
that tracked directory, all nine checks passed. This was a fixture setup issue,
not a runtime failure.

`tools/prepare_potion_fixture.py` prepares an isolated real `horizon-client-core`
fixture from a clean exact official checkout. It reuses the renderer's official
origin, clean tree, dependency identity and source-hash guards, with only its own
exact local package identity allowed. It seeds the official lock and resolves
offline; preparation does not compile. Five tests cover every built-in effect
and potion, aliases, raw/canonical names, RGB/instant values, empty/unknown
results, ordered tuples and an isolated registered custom effect. Native test
execution at exact source `7b4d6f42` passed all five tests in 0.50 seconds, with
source/lock/input identity guards passing before and after. See the
[machine-readable evidence](potion-facts-proof.json). Integrated package checking
passed in the complete Pluto package at `d039d95c`, as reported by the integration
coordinator. This adapter source hash was independently matched. The catalog
guard composes with the separately verified lossless codec through a named exact
change, rather than ignoring that function or refreshing its whole-file hash.

```sh
python3 tools/prepare_potion_fixture.py --horizon-source /approved/official/checkout --output /new/fixture
python3 tools/prepare_potion_fixture.py --verify-lock --output /new/fixture
# After a coordinated native slot, using the approved cache/profile/disk guard:
cargo test --locked --offline --manifest-path /new/fixture/Cargo.toml --lib
```

A native fact test does not verify complete consumer bundles, effect application,
resource reloads, rendered colors or gameplay. Those remain separate gates.
