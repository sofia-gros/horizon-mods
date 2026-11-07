# Bounded portable guide documents

This first generic guide group extracts restricted text data and its native
conversion. It does not install a guide screen, session registry, key handler,
packet callback, transport channel or content provider. TConstruct's page
production, item IDs, title/root metadata, layout, click regions and permissions
remain in TConstruct and are not migrated here.

## Public data boundary

The single existing API crate exports `pluto_inventory::guide::document`:

- `Document::new(pages, admit_metadata)` validates owned semantic pages. Its
  immutable page view cannot be changed without constructing a new document.
- A page contains spans of UTF-8 literal text and optional color, bold, italic,
  underline, strikethrough and obfuscation fields. None/false/true remain distinct.
  Empty spans and legacy text bytes are retained, without style normalization.
- A click action can only be a one-based page link within the document. There
  is no command, URL, file, clipboard or dialog action in the data type.
- Optional metadata is an opaque string admitted by the caller's explicit
  predicate. Rejection rejects the document rather than silently deleting it.
  PlutoLib assigns it no namespace, content meaning, owner identity or authority.

The fixed ceilings retain the existing bounded guide scale: 262,144 encoded
bytes, 512 pages, 32 spans per page, 4,096 UTF-8 bytes per text span and 8,192
bytes per metadata string. Empty documents are rejected; empty pages are valid.
Both construction and decode enforce all limits. Actual serialized JSON size,
including escaping/structure overhead, must fit the byte ceiling. Native
conversion also checks aggregate raw bytes before cloning the whole input.

`encode`/`decode` use a strict version-1 Pluto JSON document body. Unknown fields,
wrong optional-value types, out-of-range links and unknown versions fail closed.
The format is **not the legacy Tinkers payload/reply envelope**. A later consumer
migration must explicitly select its schema and request protocol. In particular,
this codec preserves a missing underline field instead of inferring underline
from a page link, and rejects unsupported actions/styles instead of dropping
those fields. No existing consumer is silently switched to these semantics.

## Native semantic conversion

The private platform boundary is `horizon_client::pluto_guide_text`.
`from_native` validates the restricted style subset and caller metadata policy;
`to_native` reconstructs native Text spans. Commands/URLs/other click actions,
fonts, shadows, hover content and inline objects are rejected on import rather
than lost or broadened. Output uses only validated page links and that same
restricted subset.

Native Text contains a separate cached source-component JSON value. This API
preserves supported rendered span semantics, not that private source cache,
translation/component structure, NBT identity or all native text features. Its
UTF-8 strings match native Text strings; invalid JSON surrogate sequences are
rejected rather than replaced. Empty styled spans are assigned directly to
native spans because Text::literal/styled would discard empty text.

No native screen or network operation occurs during conversion. Accepting a
metadata string does not grant permission to invoke an action or replace a
screen. Screen/session ownership and transport need separate capabilities.

## Official boundary feasibility

Read-only actual planning used official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. The evidence is recorded in
[guide boundary feasibility](guide-boundary-feasibility.json).

- A Game-owned session state was rejected with `E_BRIDGE_TARGET`: generic
  bridge owner types are unsupported. This is not evidence of a constructor
  control-flow rejection. No massive constructor copy or unrelated state
  carrier was introduced.
- A no-state probe resolved a RETURN on `Game::native_client_packet` and HEAD
  on `Game::open_book`. Helpers were no-ops, so this proves target/binding
  feasibility only, not a working screen or packet service.
- Ordinary `handle_packet` unconditionally calls `native_client_packet` before
  its match. Other observed call sites cover chunk/forget packets. A future
  custom-payload adapter can therefore be investigated after the existing
  native observer instead of canceling handle_packet at HEAD and skipping it.
  Full composition/order guards and actual callback tests remain required.

A supported per-world owner/session design must still distinguish native screen
replacement, world swaps, reopenings and stale remote replies. A first-span
marker alone is not ownership or generation identity. No such design is claimed
complete by this codec group.

## Validation scope

Seven pure codec tests pass, including exact byte-ceiling acceptance/next-byte
rejection, JSON escaping, Unicode/empty spans, tri-state styles, metadata policy,
page/span/text bounds and forbidden fields/actions. Four source/package checks
pass and confirm that only the conversion module is installed. Nine existing
native packaging checks also pass.

Three conversion tests use actual official horizon-client-core Text through
`tools/prepare_guide_text_fixture.py`, the real bundled API and production
converter. The fixture checks the clean official commit/origin, source hashes,
Cargo lock identities and every local dependency path. All three native conversion tests passed with exit 0 against exact official
`624c4f62`; the source/lock/path checks were repeated afterward and the production
helper/API bytes still match the tested copies. It is not a mock Text test,
a full installed-package check, or screen/packet/LAN gameplay validation.

The corrected armor/additive/provider native proofs predate this guide module
and must not be relabeled as guide validation. A later combined package cutoff
must include and check this new module before the boundary is considered ready
for consumer migration.

The document/Text group at `d161c81a` subsequently passed full library
plan/preflight/apply and client lib/bin checking in the combined official
`d039d95c` cutoff. That installed composition proof does not cover later payload
notifications or unresolved screen/session behavior.
