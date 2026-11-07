# PlutoLib design principles

These principles are the project direction. Changing them needs the owner's explicit decision, not a
convenient reading of the current code.

## What PlutoLib is

**PlutoLib is Horizon's counterpart of the Fabric API.** It is an independent mod, installed beside the
mods that use it, not a helper crate compiled into each mod. A mod calls PlutoLib's API; it does not
patch Horizon itself.

- **Fabric API like, in names, features and behaviour.** Where Fabric API has an event, registry, builder
  or lookup, PlutoLib provides the same thing under the same name (in Rust naming), with the same
  semantics, so that a Fabric mod developer can port or write a mod with little relearning. Examples of
  the shape: `ServerLivingEntityEvents.ALLOW_DAMAGE`, `UseItemCallback`, `PlayerBlockBreakEvents`,
  `ScreenHandler` / `ExtendedScreenHandlerType`, `BlockApiLookup`, `ItemTooltipCallback`,
  `ServerTickEvents`, `ResourceManagerHelper`, `LootTableEvents.MODIFY`, `FabricItem`, `FabricBlockSettings`.
- **Why Fabric API and not another model.** Horizon Mixins is expected to offer a NeoForge-like API. PlutoLib
  deliberately offers the Fabric-like one, so both communities of mod developers are served and the two
  layers do not compete.
- **One PlutoLib version per Horizon version**, versioned like Fabric API (`<PlutoLib version>+<Horizon
  revision>`). When Horizon changes, PlutoLib is updated and released; mods built on PlutoLib should need
  almost no change. Absorbing upstream change is PlutoLib's job, not each mod's.
- **General purpose, not Tinkers-only.** Future mods by anyone (a multi-page chest, a Create-like or a
  Mekanism-like mod) must be able to use it. Do not shape an API around one mod's internals; shape it like
  the Fabric API equivalent and let Tinkers use it.
- **Scope grows with need.** Fabric API's features are imitated as completely as practical, but only the
  parts the ported mods currently use are built. Start with what Tinkers needs, keep each module's boundary
  and naming where Fabric API would put it.

## What a mod looks like on PlutoLib

A mod declares a dependency on PlutoLib, registers content through PlutoLib's builders and registries, and
subscribes to PlutoLib's events. A mod does **not** ship Horizon Mixins injections of its own, and does not
name Horizon internals. How PlutoLib attaches to Horizon (Mixins injections, guards, adapters) is PlutoLib's
private implementation and is where upstream churn is absorbed.

## What this means for the current code

- `tconstruct/` still contains about 290 legacy direct injections into Horizon. They exist because the first
  port predates this direction. They are frozen by `tools/forbidden_api_baseline.json` (may shrink, never grow)
  and are to be replaced, feature group by feature group, by PlutoLib API calls.
- `pluto-lib/` today is closer to a set of Tinkers-driven adapters than to Fabric API: it pins official
  source hashes with composition guards, has no independent versioning or packaging as a separate mod, and
  lacks modules such as a screen-handler API. Closing that gap is the main engineering task:
  1. design the API surface by mapping the legacy hooks onto Fabric API modules
     (`fabric-api-design.md`, `fabric-api-hook-map.md`);
  2. package PlutoLib as its own mod with its own version;
  3. port it to the current Horizon (re-review the guarded seams);
  4. migrate Tinkers feature groups onto it and remove their injections.
- New code must not add direct injections to a mod. If an API is missing, add it to PlutoLib first, shaped
  like its Fabric API counterpart.
