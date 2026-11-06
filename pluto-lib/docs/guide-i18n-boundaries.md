# Guide and UI localization boundaries

Source audit, 2026-11-06. This is not an installed localization feature or a live-screen test. The companion JSON pins the inspected source bytes. Guide authoring API input `50336c4e` remains unchanged; its integrated CI result is separate.

## Official language behavior to preserve

- Official Horizon `624c4f62` loads `assets/<namespace>/lang/en_us.json`, then the exact selected language code, merging keys across every namespace and resource source. Later sources override within each namespace. It also applies the native deprecated-key handling. There is no general regional-language fallback chain. Some bundled Horizon strings have explicit aliases; those are not a rule for other mods.
- The language picker requires `pack.mcmeta` metadata and, for a non-English language, an `assets/minecraft/lang/<code>.json` file in a source. A mod-namespace language file can contribute translations to a selected language without making a new locale selectable.
- Missing `lang::get` returns the key; `get_or` returns the caller's fallback. `Names::display_get` first reads the current language, then the pinned English name map. These contracts are not interchangeable with a whole-file `resources::highest` lookup.
- Native JSON parsing includes lenient Gson-compatible input, primitive values and deprecated keys. A bounded JSON/TOML authoring format may normalize to native JSON, but must not silently replace this runtime parser or change resource priority. TOML is not loaded by the inspected native language loader.
- In-game selection calls `Game::load_language`, which swaps the language, refreshes the remote language map, rescales chat and clears creative-name results. Startup selection has its own language-load path. Resource reload checks the selected code again at apply time, so a selection made while the reload was running wins.
- `Text::translate` resolves its spans immediately, while also retaining a component source for serialization. `Font::split`, `Text::chars` and stored `BookScreen::View.pages` consume those existing spans. A language-map change alone does not rebuild those pages. The bounded Guide Document codec preserves literal spans and supported styles, not the native private translatable-component identity.

## Screen-by-screen status

| Surface | Existing connection | Missing or incompatible boundary |
| --- | --- | --- |
| Native language selector and ordinary draw-time lookup | Native selection, reload and fallback exist. | No new Pluto reload event or multi-client locale ownership has been proved here. |
| Native book controls | `draw_book` obtains translated control labels during drawing. | Retained page content is separate; this does not prove automatic page retranslation. |
| TConstruct builder/station information | `Names::display_get` is called while producing display lines, so those key lookups consult the current language. | Some notices remain literal English. Fixed geometry, font coverage, clipping and live redraw were not tested. |
| TConstruct smeltery labels | Draw-time `display_get` resolves title and inventory keys. | Remote machine identification separately compares English menu titles. Translating the protocol title alone would break identification. |
| Local TConstruct Guide content | Some item/material/stat/modifier strings use current-language lookup when pages are generated. | Book labels and page resources explicitly use `en_us`; titles and notices include English literals. Existing pages are not regenerated on language change. Rebuilding changes pagination, links and history. |
| Guide cover and reply identity | Existing code recognizes English first-title prefixes for six books. | Wire `book_id` validates those prefixes and drawing selects cover/page sprites with them. Translating only the title can reject replies or select the wrong cover. A consumer-owned stable book identity and compatible legacy handling are required first. |
| LAN Guide response | Existing request identifies hand and optional sequence; the server generates literal pages, and the client replaces retained pages. | The handler does not use `guest.info.language`. Official guest locale facts exist, but are not connected to this handler. Request/response has no locale or translation-component contract; current lifetime/reply authority and payload-size blockers remain. |
| Remote crafting/station/machine screens | Existing title comparisons distinguish station kind, capacity and machine type. | Display translation must be separated from those machine-readable decisions before changing the title text. |
| Native recipe-book search (recipe audit cross-reference) | Search construction uses current-language item tooltips. | The view cache key does not include language, and `load_language` does not explicitly invalidate it. UI transitions may rebuild the view; stale results are a test requirement, not a reproduced bug here. |

The source audit does not claim every UI string is covered. In particular, user-authored names remain content, not translation keys; stored NBT identity and invalid UTF-16 preservation must not be changed by a display conversion.

## Shared versus consumer responsibilities

PlutoLib can provide a thin current-language lookup/format entry over the one native language service, and bounded authoring conversion into the official resource layout. It should preserve the existing dependency direction and use one service, not a second global locale/catalog. Returning an owned translated string is a snapshot, not a promise that retained content follows future settings changes. Any new locale validation, authoring schema, limits, output conflict policy or TOML dependency must be explicit and tested separately.

TConstruct retains translation keys, source-book formats, content, fallback wording, layout, stable book/machine identity and legacy protocol policy. Rebuilding an open Guide needs the still-unproved retained-screen capability. The current input-only book operation is not authority to refresh an overlaid retained screen. Guest locale storage alone is not permission to mutate the process-global client language during server page generation. The native test-only `lang::scoped` helper also leaves bidirectional state process-global, so it is not a proven per-guest rendering solution.

No LAN handler, page generation, title, registry or screen state is changed by this audit. Existing LAN functionality has not been disabled.

## Reproducible next checks, all unrun here

1. Use official `Language::load` with an in-memory `ResourceSource` to verify selected-code/English merge, missing keys, multiple namespaces/sources, native malformed-value handling and picker eligibility. Native `lang.rs` already has focused tests such as `layers_selected_language_over_english`, `languages_are_listed_without_reading_their_files` and scoped-language tests. Their existence is evidence of a fixture route, not a test result from this audit.
2. Test any authoring JSON/TOML normalization independently, then load its exact output through that native loader. Preserve unknown/missing distinctions and make rejection/limits explicit. Do not infer renderer or pack-installation proof from a pure parser test.
3. With actual native Text, establish that an already-created translated value retains old spans after a scoped language change and that explicit re-creation uses the new language. This is a CPU text fact, not a live-screen refresh test.
4. Before consumer migration, lock old English title/wire behavior, add a stable identity path without removing legacy support, and test locale changes between request and response, overlays, identical reopen, resource reload, pagination and delayed replies. Actual Client/UI/LAN proof remains unavailable; the excluded retained-book prototype has not yet passed an actual Game typecheck.

See [Guide API usage](guide-api-usage.md), [LAN connection status](guide-lan-safe-connection-status.md), and [the proposed native typecheck handoff](guide-native-typecheck-ci-handoff.json) for the separately bounded existing work.
