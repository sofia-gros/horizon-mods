# Install the native library

PlutoLib's development package is one source-only `.hnmod` shared by native mods. It contains the Creative and item APIs, a two-slot [inventory move planner](inventory.md), [advancement data scopes and inventory events](advancements.md), component transport, Tool rules, and durability tooltip and bar adapters. It also includes an experimental [world-fluid service](world-fluids.md), whose selected-entry validation is separate from whole-bundle compatibility. It does not contain a consuming mod, demo hooks or mod assets.

## Build the bundle

From `pluto-lib`, with Python 3.11 or later:

```sh
mkdir -p .work
python3 tools/package_native.py .work/plutolib.hnmod
```

The native mod ID is `pluto_native_creative`; its Mixins source is under `mixins/pluto_native_creative/`, and its Cargo crate is `pluto-api` (imported as `pluto_api`). The output must not already exist. Python builds the archive; it is not needed by the native API. The component source directories remain available for development, but the generated archive combines them as one official Mixins bundle with one `pluto_api` crate and one shared inventory-planning crate.

The two crate roots are explicit. Their `Cargo.toml` files and regular `.rs`
files below `src/` are included deterministically, including nested API modules.
Source symlinks are rejected. Build output, tests outside `src/`, demos and
unlisted non-Rust resources are not copied. Adding an `include_bytes!` resource
requires extending and testing that packaging contract explicitly.

## Use it with a mod

The current recovery candidate requires the guarded source-tree staging route
below. Do not install it directly through the ordinary official launcher yet:
that launcher does not execute this candidate's required complete-plan guards.
A matching `.hnmod` archive alone does not establish safe hook composition.
A consumer may also need its own content package. All participating bundles must
target the same Horizon and Mixins revisions.

Use `pluto_api::creative` and `pluto_api::item` as described in [Creative preparation](native-creative.md) and [short item use](item-use-b2.md). Tool functions currently use host-side paths shown in the [combined demo](../experiments/unified-target/README.md). Do not enable the individual six component bundles alongside this archive: doing so would register the same hooks or crate twice. Do not combine the older Creative paging or item-network adapters with it.

The current development candidate targets official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. It is not a release or full integration
claim. See the [recovery baseline](official-upgrade-20261105.md) for the exact
cutoff and promotion gates, and [compatibility](compatibility.md) for historical
build and gameplay evidence. Focused API checks do not certify all installed
native hooks, the full launcher, Windows or LAN behavior.

The development package currently has three per-world owned states on the
supported `ChunkMap` constructor: transient landing context, advancement tag
snapshots/custom index cache and one-shot saved recipe knowledge. All clear
with their owning world. They do not
inject fields into public `StatusFlags` or constructor-free `AdvState` types.
At the exact `b19b35a1` cutoff, the complete packaged declarations, constructor
coverage, source guards, application and native lib/bin type-check passed. See
[the complete-library proof](shared-foundation-validation-20261105.md).
This does not establish TConstruct landing behavior, tag producer activation or
gameplay/save/LAN compatibility. Earlier experiments with other state targets
remain historical evidence, not the current implementation.

The advancement inventory adapter is default-off. Consumers must explicitly
request full-component inventory events during initialization before the first
inventory event; installing PlutoLib alone retains the native event path.
See [the activation contract](advancements.md#inventory-event-activation).
Step 3 consumer activation is not provided by the shared foundation.


## Mandatory development preflight

`tconstruct/tools/stage.py` now plans **all** installed bundles together, then
runs `pluto-lib/tools/check_native_plan.py` before applying any changes. CI and
the Windows development helper use this same path. The guard checks protection
ordering, the exact official damage-caller source and reviewed helper bodies,
and rejects interior hooks that copied callers would bypass. It also requires
exclusive ownership of the two copied advancement targets and checks their
exact official fingerprints, bindings and packaged helper bytes. The read-only
client payload service also requires its exact reviewed native observer RETURN
and rejects competing hooks that could bypass or reorder observation. Its
source, bindings, resolved function and helper bytes are checked by the same
mandatory preflight. The same complete-plan entry point also checks the two
placement veto targets, current-book native page-turn composition, and the exact
private Wire source/declaration and additive reader/comparator modules. Deny-only
crafting admission and resource-owned terrain preparation also have exclusive,
source-pinned boundaries in the same preflight. Explicit cloud authority/publication
and station ingredient parsing add two further checked boundaries. The terrain
cell dispatch adapter adds a twelfth mandatory source/composition group. A failed guard
stops before application. Do not validate Pluto in isolation and then append
consumer bundles to that result.

Keep the matching PlutoLib source checkout and its installed package together.
The source-tree guard verifies the copied helper bytes embedded in the actual
plan; it does not execute code supplied by an arbitrary package. Install the
`rustfmt` component for the pinned Rust 1.95.0 toolchain, because exact reviewed
source regeneration is part of preflight. Missing tools or mismatched sources
fail closed.

The standalone equivalent, after the official CLI has generated a complete plan,
is:

```sh
python3 pluto-lib/tools/check_native_plan.py complete-plan.json pristine-official-horizon
```

This remains an experimental guarded staging workflow, not a completed launcher
integration. Native type checks do not replace gameplay, save/reload or LAN
validation. The [complete-library proof](shared-foundation-validation-20261105.md)
records its exact cutoff; later source additions need another proof.

The read-only [potion/effect catalog adapter](potion-facts.md) exposes existing
native registry facts through `horizon_client::pluto_potions`. Application and
source-version policy stay in consuming mods; its focused and integrated proof
scopes are recorded separately.

The [explicit cloud construction](cloud-construction.md) group validates shared
descriptors and stages native objects without spawning or choosing an actor.
It is separate from playable cloud effects and requires its own host proof.
