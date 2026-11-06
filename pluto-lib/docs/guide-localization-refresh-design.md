# Localized retained Guides and LAN replies: connection design

Design only, 2026-11-06. Nothing in this document installs a refresh hook, changes a legacy packet or translates an existing screen title. The `Key` source candidate at `76228074` / published `a630d1ab` is a separate, much smaller feature: it resolves text when its caller asks. Its native tests do not prove this design.

## Verified source constraints

The [UI audit](guide-i18n-boundaries.json) pins official Horizon `624c4f62` and the unchanged consumer at `50336c4e`.

- `lang::set` replaces the language map. `Game::load_language` and resource-pack apply are different entry paths. A reload can replace translations without changing `lang::code`; comparing only the code misses that case. The native language map is process-wide, with a test-oriented thread scope, not a proven per-guest display context.
- `Text::translate` stores resolved spans and a translatable component source. Retained `BookScreen::View` drawing uses spans; it does not re-resolve the source component. Retained arguments are also snapshots. Resolving a new `Key` does not update a previously stored Text.
- `source_pages` constructs wrapped pages, chapter indexes and link destinations. The modifier index even searches for entries by equality with displayed source titles (`source-materials-book.rs:689–705`). Translating titles can therefore affect both book identity and intra-book routing. A full localized rebuild is not a string replacement.
- The legacy response replaces a retained non-lectern book beneath overlays, preserves the cover sentinel and clamps a normal page index. It does not rewrite the stored back-history index. Turning language-change refresh into an input-only action would remove existing overlay behavior.
- The current Guide request carries hand and optional sequence, not locale. The host has guest language in `ClientInformation`, but this handler does not use it. The wire decoder returns literal pages, not translation keys and arguments.
- Official `horizon_protocol::mod_net` advertises only support for generic mod messages, not support for a particular Guide revision. It caps message data at 30,000 bytes. It cannot directly replace the existing Guide envelope, which may be 262,152 bytes. No implicit chunking, truncation or broader payload cap is proposed here.

The additional transport evidence is official `crates/horizon-protocol/src/mod_net.rs` at the same Horizon pin, SHA-256 `66e85c0a3ab916c7fc6a0ea7e4b9aa9925d8338ede924a99fa9eb798d9654173`. Only its source was inspected; no protocol test was run in this design pass.

## Keep the two operations separate

**Ordinary labels:** retain a `Key`, rebuild its arguments if needed, and resolve at the consumer's existing admitted display boundary. The native service performs lookup and formatting. This can follow the current language without a second catalog or an automatic event framework. It neither changes nor owns a retained screen.

**Retained Guide rebuild:** preserve the current pages while preparing a complete replacement. Apply it only through a still-required owned retained-opening capability, after validating current connection/opening, book identity and the request/localization inputs. An observed stamp, root marker, matching title, content equality or pointer is not that capability. The excluded native storage wrapper remains a proposed host extension and still needs actual Game type proof and owned-open acquisition.

## Minimum identity separation before consumer translation

1. **Book identity:** use the source item ID, such as `tconstruct:puny_smelting`, as consumer-owned identity acquired when opening the book. Select cover/page sprites from this identity, not translated first-title text. This value must be bound to the actual owned opening, not trusted merely because it appears in received metadata.
2. **Content anchors:** retain source chapter/entry identifiers before localization and pagination. Map generated pages and link destinations from those identifiers, not equal translated headings. The title remains presentation content.
3. **Localization inputs:** distinguish requested locale from the resource/catalog snapshot used to build pages. A same-code resource reload must make older prepared content stale. Native hook coverage or a safe catalog-snapshot token still needs proof; no new global generation counter or guessed event is installed here.
4. **Pending work:** compose the existing checked request tracker with the owned opening and consumer's localization inputs. A new request for the same opening supersedes older work, including work from a previous language. The tracker's in-process token has no wire encoding and must not be cast or serialized as if it were an existing protocol field.

Each of these is different from the page's mutable revision. A harmless page turn or conservative mutable borrow must not by itself invalidate a still-current request for the same owned opening. Conversely, an identical reopen or reconnect must invalidate it even if every displayed value matches.

## Compatibility-preserving protocol boundary

The unchanged legacy path remains in place until a replacement has actual proof. The new implementation must continue accepting the existing one-byte/nine-byte requests and body/eight-byte-prefix response forms where they are already supported. It must not rename legacy English titles while the old decoder uses them as identity.

A future localized protocol needs a separate explicit capability/version contract. Generic Horizon mod-message support is not sufficient evidence. Do not send a new request format to an unconfirmed peer and assume silence authorizes a fallback, and do not send parallel old/new requests whose delayed replies can overwrite one another. Unknown peers retain the legacy behavior and its documented localization limits. This is compatibility preservation, not a claim that old peers become localized.

For an explicitly negotiated revision, the consumer needs a stable book ID, a fresh request correlation value and the requested locale. A reply must identify the request it answers. Two content strategies are materially different and are **not selected or activated by this design**:

- Host-localized literal pages preserve the current host-side data source, but require explicit locale lookup rather than changing the process-global language. Server fallback/resource availability and server-versus-client font pagination remain limits.
- Locale-neutral facts plus client-owned translation keys/source content let the client resolve its own language and font layout, but require a larger schema and an authority/admission review for recipes/material facts and client content. This is not equivalent to merely replacing the current callback.

The exact bounded envelope, legacy decoder policy and 262,152-byte edge case must be handled as one reviewed admission/consumer contract. The existing read-only observer's 262,144-byte ceiling is not raised here. No command/URL capability is added to the Guide codec.

## Refresh and commit order to prove

1. A proved native selection/reload boundary marks the owned presentation's localization inputs as changed. Old pending localized work is superseded; current pages stay visible until a replacement is ready. This does not yet claim that the boundary exists as an installed Pluto event.
2. Build replacement pages and a validated anchor map with explicit source/locale inputs. Failure leaves the old pages intact and reports the failure rather than hiding content or partially rewriting links.
3. After native packet handling, take one coherent subscriber snapshot and invoke the approved observation phase. Revalidate the owned connection/opening, stable book identity, localization inputs and pending request **after** callbacks and immediately before commit.
4. Consume the exact matching request at the chosen commit boundary, then perform one conditional retained-slot update. Duplicate or stale replies cannot reapply. An exception/panic after effect attempt is outcome-unknown, not permission to retry.
5. Preserve the cover sentinel. For a normal page and back-history, prefer proved source-anchor remapping when defined. The legacy numeric clamp and unchanged history are the current oracle; choosing a different fallback when an anchor disappears is an explicit consumer behavior change to review, not something the shared library decides silently.
6. Overlay dismissal must observe the updated retained pages, not restore a stale saved view. Actual creation, close, replacement, guest save/restore, cancelled-command restoration and world/connection reset paths remain part of the lifetime proof.

## Staged acceptance, not passing results

| Gate | Required cases | Present status |
| --- | --- | --- |
| Thin Key/native Text | Selected/English/missing fallback, source component and styled arguments, old snapshot/new resolution, borrowed fallback, actual public imports | Six native tests and example supplied; source guards pass; native execution queued with World/Recipe. |
| Existing language settings | Selection during resource reload; same-code changed resources; no false per-guest/global-bidi claim | Source behavior audited; no live UI test. |
| Stable identity/anchors | Six books, translated equal titles, title change, duplicate chapter labels, removed entries, cover selection | No consumer change or runtime test. |
| Legacy compatibility | Exact old packet bytes, old peers, maximum body/envelope, existing overlays, cover/clamp/history | Source oracle retained; no new connection test. |
| Localized delayed replies | Language A→B before A reply; same-code reload; B reply then duplicate A/B; close/identical reopen; same-key reconnect | No owned native reply connection; not tested. |
| Callback and commit races | Observation changes opening/localization; wrong/malformed/oversized input; pending consume once; post-attempt panic | Generic model requirements only; no live result. |
| Actual native storage/UI | Actual Game typecheck of excluded prototype, then real Client/window/resources and all restoration paths | Typecheck is a separate proposed CI scope. A headless or renderer-less fixture cannot stand in for live UI. |

The first gate can be proved independently without enabling any later gate. No existing Guide feature is disabled to make the new API appear complete.
