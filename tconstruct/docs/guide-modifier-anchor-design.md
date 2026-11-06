# Modifier links: a bounded source-identity proposal

This is an uninstalled consumer source candidate, not a validated runtime migration.
The original oracle pins local `6c363d99fc1455cc4911f664f7b60639f2281f6f`, published
as `37a5ee6ab59da5640a1138cc3ff2889fad8976b8`. The candidate changes only
`source-materials-book.rs` and its existing `source-materials-book-index.rs` helper
against local `20080e461f4fb81046e9f338693803864b3b0761`. Module entries, drawing,
wire/request/response code and legacy English book identity remain byte-identical.

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

The candidate retains the chapter key and records only
the six reviewed data paths' first actually generated page offsets while building
that chapter. Final modifier links use those offsets, while their displayed
labels and ordering continue to use the existing source titles. No table needs to
be exposed outside this one `source_pages` invocation, serialized into the book,
stored in Game or attached to a pending LAN reply. No unused all-book registry is
proposed.

## Proposed rules and intentional differences

- Use the exact chapter key `modifiers`, not its displayed heading. The old
  requirement that the `modifiers` language label exists is preserved. A different
  chapter with the same heading no longer receives modifier links. A changed
  heading does not erase the real modifiers chapter's links.
- Use the exact relative data path, without case/path/title normalization. A file
  with the same basename under another directory is a different entry.
- The first occurrence with a nonempty source title and an actually generated
  first page whose first span equals that title wins for a duplicate data path.
  An empty/malformed earlier occurrence does not claim the offset.
- Zero generated pages mean no destination. The old search can instead borrow
  an unrelated page with the same title; the proposed rule deliberately does not.
- Graphic plus overflow pages refer to the first generated page, as the existing
  first-title search does with the fixed normal input. Pagination still belongs
  to the existing native Font calls; the anchor rule adds none.
- Missing source title/index admission still prevents the corresponding link.
  Source files are read again by the same final-link code; the oracle models
  those final title facts separately from generated pages. The candidate keeps
  those reads and does not imply a coherent immutable resource snapshot exists.
- A notice prepended after generation must shift recorded offsets by its actual
  generated page count. A notice with a coincidentally equal title must not become
  a source destination. In the fixed consumer, notice insertion is not active for
  Materials' modifiers chapter; these cases exercise the proposed rule, not an
  observed native route.
- Two equal display titles keep their separate source destinations. The old rule
  intentionally differs by pointing both to the first equal title. An entry whose
  title equals the index heading likewise must not link to the index itself.

The first oracle at `a8331d13` allowed a generated page without a first span to
acquire an anchor. This candidate deliberately narrows that proposal: empty source
titles, absent first spans and first spans different from the source title acquire
no linkable anchor. Final link source titles must also remain nonempty. All actual
page bodies are retained; only the link is rejected. The old title search can borrow
an unrelated equal title or even an empty-title span, so that accidental behavior
is not preserved. The revised oracle demonstrates this difference explicitly.

## Oracle and evidence

`tests/book-wire/modifier-anchor-oracle.json` pins 22 complete source files and
stores the exact old resolver block. Sparse resource files are read only from the
pinned local MOD Git object. No engine fork or unknown resource tree is consulted.
`test_modifier_anchor_oracle.py` contains independent old-predicate and proposed
source-offset models over already-generated first-span facts.

Thirteen Python oracle tests pass, including all 729 positive page-count vectors (1–3 pages
per each of six fixed entries), duplicate titles, duplicate paths, zero/missing
entries, index collision, chapter-label collision, notice prepend and graphic
overflow. The fixture is capped at 32 source chunks and 256 output pages; these
are test budgets, not proposed runtime content limits. Python integer addition
does not prove Rust overflow handling. The eventual caller must retain its checked
`usize`/`i32` conversion and destination addition.

The old source-pin test reads the immutable baseline Git objects even when the
working candidate has changed. Six additional Python source guards use
`modifier-anchor-runtime-scope.json` to reconstruct both complete pre-change
source files by reversing the reviewed edits. They also check unchanged packet,
draw and manifest bytes, exact Font-call lines/order, source re-reads, label
admission, checked destination arithmetic and the six-entry map. The four earlier
BookId guards also pass after first reversing this separately hash-checked
follow-up, then their original edits: 23 Python tests pass in total. Their first
run against the later source correctly failed the old whole-file hash; the layered
reconstruction fixes that test composition without weakening either boundary.
This is not execution equivalence proof.

Five Rust helper tests passed at dedicated CI head
`93424e7914f8e31ca289beb29254c4e7740c4f82`, run
[42187284494](https://github.com/sofia-gros/horizon-mods/actions/runs/42187284494).
They cover duplicate display
titles, first admissible chunk, exact paths, notice shifts and atomic overflow
rejection. The exact production helper was included as a module, without an
extracted algorithm or mock Text. Five separate BookId tests passed against the
same sealed executable, with 21 total discovered tests and sixteen filtered in
each process. Twelve runner checks also passed. Source and binary seals remained
unchanged; [the receipt](guide-identity-ci-result-20261106.json) records the exact
scope. No native resolver, Text, Font, Game, book-wire fixture, Mixins plan,
package application or rendered screen was executed for this candidate. None of
these tests belongs to the successful 22-test language/Key/item CI.

## Activation gates still open

1. The helper tests and bounded source review are complete. Integrating this
   source into a verified consumer build still requires the native gates below.
2. The candidate records after graphic/body/overflow generation, including the
   former graphic-only early continue, and shifts by actual later prepend count.
   Full inverse-source hashes and Font-call ordering guards pass, but actual native
   generation still needs a test; source comparison is not its substitute.
3. Compare fixed-source generated links and pages against the old caller using
   an actual native Text/Font fixture, with separate collision assertions.
4. Check the complete consumer's official plan/apply/type composition. Existing
   Guide response hook conflicts are not removed by an internal anchor map.

This work neither localizes existing Guide pages nor enables a retained-page
refresh, network revision or lifetime token. Source anchors alone cannot prove
that a delayed reply still belongs to the same opening. Existing LAN behavior is
left intact while those separate boundaries remain unresolved.
