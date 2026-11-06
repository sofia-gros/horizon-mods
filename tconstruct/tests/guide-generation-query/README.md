# Guide source-page generation through existing Pluto queries

Source-only consumer candidate, separate from the passing Font12 input and from
latest retained-owner CI c5e4094b. The exact starting MOD source is
8b32beceb00333f93fce251b4915f431b2c071ef. The prior Font12 proof is
the separate 624c/59c input, not evidence for this delta or latest f4dc. No new Pluto API or ordinary hook/state entry.
No Rust, actual Font, Game, installed package, UI or LAN test has run on this delta.

The existing `split_styled(Game, Text, width, rows)` delegates to a private
consumer support module using the existing `split_raw` API. The old native
sequence remains: one `resolve_legacy`, one native Font split, then
`chunks(rows.max(1))`, line clones and newline insertion. No eager measurement,
second resolution, glyph preflight or post-query retry is added.

The resolved Text is admitted as a temporary one-page Document before the first
Font query. Unsupported styles/actions, page targets outside that temporary page,
size/work budgets and invalid widths use the original splitter before any cache
mutation. This is not a new acceptance rule for generated content. It neither
removes unsupported content nor renumbers links. Metadata remains opaque. The
resulting pages retain the original row policy, even zero rows and native unusual
width/metric behavior. Conversion/serialization performs additional CPU work and
allocations; these are not claimed free or performance-equivalent.

Inverse source checks restore the complete prior book generator and support
module, not just sampled lines. Draw/click, manifest, all legacy wire/response/
request files and existing Pluto helpers are pinned unchanged. This does not
connect the unsolved mutable LAN observer or retained ownership protocol. English
title identity, page refresh and translation activation are unchanged.

Six actual Font tests are prepared in `native.rs`. They include the unchanged
Font12 source for original synthetic metrics/bitmap helpers and its 12 tests.
A later reviewed fixture must seal the complete original file/API/lock graph,
compile actual official Text/Font and run the new `generation_tests::` filter
separately. No mock host type is used. The two bitmap cases compare independent
Font instances, raw CPU patch order and an actually-full atlas region. This is
still not GPU drawing, input, screen lifetime, real assets, LAN or full Game proof.
No standalone runner, workflow or fixture preparation is activated here.

Six final pure source checks passed on 2026-11-06. They verify complete inverse
source hashes and unchanged scope, not executed Rust or Font behavior.
