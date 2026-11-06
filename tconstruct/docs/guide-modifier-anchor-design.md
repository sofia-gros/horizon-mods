# Modifier links: a bounded source-identity proposal

This is an uninstalled consumer design and pure oracle, not a runtime migration.
The baseline is local `6c363d99fc1455cc4911f664f7b60639f2281f6f`, published as
`37a5ee6ab59da5640a1138cc3ff2889fad8976b8`. All production source, module entries,
pages, titles, packets and LAN behavior remain unchanged in this follow-up.

## Smallest actual caller boundary

Of the six fixed books, only Materials & You has a `modifiers` chapter. Its
`sections/modifiers.json` declares six exact data paths: `modifiers/index.json`,
`traits.json`, `upgrades.json`, `slotless.json`, `abilities.json` and `defense.json`
(the last five also have the `modifiers/` prefix). Their current English titles
are nonempty and distinct. These are consumer content identities, not generic
Pluto registrations or proof of an owned screen.

Today `source_pages` discards the chapter key before constructing final links.
It selects the modifiers chapter by equality of displayed heading and a language
label, then finds each destination by the first equal first-span title. A second
entry or an unrelated chapter with the same translated title can therefore
select the wrong destination. The new BookId helper does not solve this separate
chapter/entry issue.

A minimal eventual consumer change would retain the chapter key and record only
the six reviewed data paths' first actually generated page offsets while building
that chapter. Final modifier links would use those offsets, while their displayed
labels and ordering continue to use the existing source titles. No table needs to
be exposed outside this one `source_pages` invocation, serialized into the book,
stored in Game or attached to a pending LAN reply. No unused all-book registry is
proposed.

## Proposed rules and intentional differences

- Use the exact chapter key `modifiers`, not its displayed heading. A different
  chapter with the same heading no longer receives modifier links. A changed
  heading does not erase the real modifiers chapter's links.
- Use the exact relative data path, without case/path/title normalization. A file
  with the same basename under another directory is a different entry.
- The first occurrence that actually generates at least one page wins for a
  duplicate data path. An empty earlier occurrence does not claim the offset.
- Zero generated pages mean no destination. The old search can instead borrow
  an unrelated page with the same title; the proposed rule deliberately does not.
- Graphic plus overflow pages refer to the first generated page, as the existing
  first-title search does with the fixed normal input. Pagination still belongs
  to the existing native Font calls; the anchor rule adds none.
- Missing source title/index admission still prevents the corresponding link.
  Source files are read again by the old final-link code; the oracle models those
  final title facts separately from generated pages. A future integration must
  either preserve those reads or explicitly review the semantic change, rather
  than imply a coherent immutable resource snapshot already exists.
- A notice prepended after generation must shift recorded offsets by its actual
  generated page count. A notice with a coincidentally equal title must not become
  a source destination. In the fixed consumer, notice insertion is not active for
  Materials' modifiers chapter; these cases exercise the proposed rule, not an
  observed native route.
- Two equal display titles keep their separate source destinations. The old rule
  intentionally differs by pointing both to the first equal title. An entry whose
  title equals the index heading likewise must not link to the index itself.

Pages with no first span expose another intentional difference: path identity can
still refer to that generated page, whereas the old title search cannot. Before
runtime activation, review whether to admit such malformed/empty-title content or
exclude it explicitly. This oracle demonstrates the difference; it does not settle
the consumer policy or claim that fixed native generation produces it.

## Oracle and evidence

`tests/book-wire/modifier-anchor-oracle.json` pins 22 complete source files and
stores the exact old resolver block. Sparse resource files are read only from the
pinned local MOD Git object. No engine fork or unknown resource tree is consulted.
`test_modifier_anchor_oracle.py` contains independent old-predicate and proposed
source-offset models over already-generated first-span facts.

Twelve Python tests pass, including all 729 positive page-count vectors (1–3 pages
per each of six fixed entries), duplicate titles, duplicate paths, zero/missing
entries, index collision, chapter-label collision, notice prepend and graphic
overflow. The fixture is capped at 32 source chunks and 256 output pages; these
are test budgets, not proposed runtime content limits. Python integer addition
does not prove Rust overflow handling. The eventual caller must retain its checked
`usize`/`i32` conversion and destination addition.

The source-pin test proves that these models refer to the unchanged reviewed
inputs. It does **not** execute the native resolver, Text, Font, Game, book-wire
fixture, Mixins planner or a rendered screen. These tests are not part of the
successful 22-test native language/Key/item CI and do not expand its scope.

## Activation gates still open

1. Review the explicit differences above, especially empty first spans and
   duplicate/absent generated entries, before switching the consumer rule.
2. Record offsets after every actual output path, including the graphic-only
   early continue and later prepends. Preserve all Font calls, page bodies,
   source admission and ordering; avoid an offset recorded before generation.
3. Compare fixed-source generated links and pages against the old caller using
   an actual native Text/Font fixture, with separate collision assertions.
4. Check the complete consumer's official plan/apply/type composition. Existing
   Guide response hook conflicts are not removed by an internal anchor map.

This work neither localizes existing Guide pages nor enables a retained-page
refresh, network revision or lifetime token. Source anchors alone cannot prove
that a delayed reply still belongs to the same opening. Existing LAN behavior is
left intact while those separate boundaries remain unresolved.
