# Guide book identity mapping: source checkpoint

This consumer-only refactor consolidates the existing six-book item, resource,
legacy-title and sprite mappings. It does not enable localization, new packet
fields, a retained-screen owner or a reply capability. The generic Pluto language
API does not contain any of these TConstruct identities.

## Preserved callers

- `source_pages` resolves the exact source item ID and receives the same root and
  English title. Its remaining body, pagination and index generation are unchanged.
- Drawing performs the same non-lectern/root-marker admission, then delegates
  sprite selection. An unknown or empty root-marked title still selects Materials
  sprites. No font query or drawing operation is added.
- The private wire `book_id` performs the same root/first-span checks, then calls
  the distinct legacy wire mapper. Unknown titles still return `None`. The short
  internal labels remain the same; no enum value or new identity is serialized.
- English prefixes still require the same immediate newline. Case, whitespace,
  CRLF and localized titles are not normalized into accepted identities.

The new `BookId` values are content facts, not proof of ownership. The old title
dependence remains explicit in `legacy_draw_sprites` and `legacy_wire_id`. A future
localized path must bind a source item identity to an actual owned opening rather
than trust arbitrary received metadata or reuse the legacy prefix as authority.

## Exact scope and evidence

The base is local `bbb4948b98672ab3cb3a59178eab103b51229902` (Guide Key RGB correction).
Five runtime paths change: one new pure helper, three consumer callers and one
module declaration in the existing TConstruct manifest. The existing book-wire
fixture gains the sibling module include it needs.

`tests/book-wire/identity-refactor-oracle.json` records the old/new edit at every
changed existing path and the complete previous-file SHA-256. Four Python source
checks pass, including reversal of every edit back to those exact previous bytes.
Thus packet encoding/decoding bodies, sequence, limits, root guards, history and
all other source bytes in those files are unchanged. This is source evidence,
not execution equivalence of the installed game.

Five pure Rust tests are supplied for exact item/root/title mappings, legacy
prefixes and suffixes, near misses, unknown/translated-title asymmetry and exact
item IDs. They have **not run** at this checkpoint. The pre-existing book-wire
fixture uses a mock Text type; any future result from that fixture must not be
reported as an actual native Text, Game, UI or LAN test.

No Rust build, official plan/apply, full consumer type check, live screen or LAN
test has run for this refactor. Existing consumer/Pluto hook-composition blockers
remain. The independently prepared language/Key/native-item 22-test CI input is
not changed or expanded by this branch.

See the shared [localization refresh design](../../pluto-lib/docs/guide-localization-refresh-design.md)
for the still-unproved lifetime, localization, anchor and protocol boundaries.
