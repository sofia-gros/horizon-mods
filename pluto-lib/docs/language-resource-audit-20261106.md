# Language resources: official source audit

Status: **source-only**, 2026-11-06. No new API, hook, dictionary, package execution,
Rust test or live translated screen is supplied here. Declaration and authoring CI
inputs remain frozen. Exact source identities and locations are in
[the audit JSON](language-resource-audit-20261106.json).

Targets: official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, official
Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`; Pluto/consumer inspection is
local `8715b1232c0fca133fe012f35830e2108e653d59`, published equivalent
`89744814b8d536a117e82f8ea3c8cdc9798faa95`. Every listed file matches its committed
bytes. This is source evidence, not a claim that all those code paths executed.

## One existing owner

`horizon_client_core::lang` owns the resolved language, and `horizon_client::lang`
re-exports that owner. `Language::load` reads `assets/<namespace>/lang/<code>.json`.
It starts with bundled Horizon strings plus supplied sources, loads `en_us`, then
the exact selected locale. Namespaces are sorted and are the outer loop; sources
are the inner loop. Thus source priority applies within a namespace, while a later
namespace can overwrite a key from a higher-priority source in another namespace.
Keys are not automatically prefixed. Consumer-specific key names prevent accidental
cross-mod collisions; a namespace folder alone does not isolate them.

Missing selected values retain English. `get` falls back to the key and `get_or`
to its explicit fallback. There is no automatic `ja_jp` to `ja` fallback. The stored
code remains the requested code even when only English was found. Deprecated key
removal/renaming runs after merging and can remove previously available values.

Only `.json` language paths are requested. The direct loader supports native
lenient JSON, including primitive numbers/booleans converted to text; an invalid
non-primitive value stops the rest of that file. The resource-pack merging layer
first parses strict JSON objects. Therefore accepting a commented file directly
is not evidence that the same file survives pack merging. TOML manifests and
Mixins configuration do not imply TOML language resources are supported.

## Packaging, conflicts and actual priority

Official content discovery finds sorted `.hmod` files/folders and then explicit
dev paths. Content must be enabled (or dev), have `registries` capability and pass
owned-namespace/path/size checks. Two content mods owning one namespace conflict.
A `.hnmod` source bundle is not itself a discovered content package. Native bundle
`data/` is staged as `native-data/<mod>` for compiled bytes, not automatically as
client assets. A combined `.hmod` can carry native code plus the content manifest,
`assets/` and `data/`.

Resource packs are overlaid first, server packs above selected packs. Their
same-path language JSON files merge by key, with higher layers replacing lower
values and filters suppressing lower resources. **The content-mod overlay is
then built over that pack root.** Ordinary Mod language assets are written as
whole files. An overlapping resource-pack translation path can therefore be
replaced by the Mod file, including keys the Mod file lacks. This audit preserves
that official behavior; it does not promise Fabric-style override priority or
silently fix it in Pluto.

The existing TConstruct packager keeps tracked language output first, including
its English additions, and copies original-language files only at missing paths.
That is existing consumer policy, not a generic Pluto translation registry.

## Locale discovery and reload

`available` uses `pack.mcmeta` language metadata, but a non-English locale also
requires `assets/minecraft/lang/<code>.json` to exist in a supplied source.
A custom-namespace-only locale does not meet that menu-discovery gate. Supplying
translations for an already supported locale is the smaller first slice.

On reload, staging reconstructs the pack/content root and resolves the current
language. Application rechecks the selected code, reloads it if selection changed
mid-stage, installs it through the official owner, and refreshes remote language.
The existing resources-reloaded notification is after successful application,
not an initial-startup or failure callback. No automatic file watcher was found;
content caching uses its existing source metadata stamp. UI text already reduced
to a String still needs the normal UI update/re-resolution contract, owned by the
separate Guide investigation.

## Smallest connection and remaining gates

Keep `assets/<mod>/lang/en_us.json` and `ja_jp.json` in the consumer's content
package. A thin client-facing Pluto lookup/format facade, if introduced, should
only delegate to the official `lang`/`Text` owner. Do not create a parallel locale
selection, dictionary or reload registry. The public API and internal host adapter
can be separate logical layers while using one underlying owner. TConstruct uses
Pluto; mods not using Pluto may use official Mixins directly.

The first visible consumer must call that real lookup/translation path for an
actual item/menu label, then be checked under both selected locales. A code sample
or successfully packaged JSON is not that display proof. Recipe owns key/format
compatibility; Guide owns locale selection and UI refresh. This audit owns resource
loading and collision evidence only, not a runtime change.

Pending bounded tests must use the actual official loader and overlay functions:
English fallback, cross-namespace priority, custom-locale discovery rejection,
pack-versus-Mod whole-file replacement, direct-lenient versus pack-strict inputs,
and real UI re-resolution. No missing-resource skip may count as success. Local
native work remains held for disk protection; no compiled tests were run here.
