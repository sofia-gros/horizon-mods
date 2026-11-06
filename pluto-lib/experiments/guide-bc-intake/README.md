# Guide B/C intake, preserving PR945 navigation

Source-only merge, not installation or runtime success. The two generated files are produced from exact public commits recorded in inputs/intake. They are not installed by this directory. The coordinator owns the integration branch.

Navigation base: `d4d798fbbdc9dbbe0f81adb0e4074c798737e5ad` (#945). Consumer B/C source: `d9f8e6ba50fe7b1d9a3fc457bafa1650d3da8bd5` (parent `4fee3bee7dc5d151bd31bd977316329a17f3564a`).

## Runtime intake

Copy these six exact blobs from d9f8 under `tconstruct/mixins/tconstruct/`:
- source-materials-book-text.rs
- source-materials-book-draw.rs
- source-materials-book-identity.rs
- source-materials-book-wire.rs
- source-materials-book-index.rs
- source-materials-book.rs

For horizon-mixin.toml and source-materials-book-click.rs apply only the included patch to d4d. The patch adds two consumer modules and the exact B lazy query loop. Every navigation injection, input/key/cover control, arrow turn call, link handling and history tail remains from #945. Other manifest tables and all existing injections are unchanged. The exact output hashes and full original Git blob inventory are in intake.json.

All shared `pluto-lib` runtime remains common035273-owned. Do not copy a full old source tree over that bundle. The d194 generation delegation, failed Font6 candidate, retained-storage/native-opening patches and unproved LAN bridge are excluded, not made active by this merge.

## Test/document intake and evidence

intake.json lists exact paths/blobs for `tconstruct/tests/guide-text-migration/`, `guide-identity-proof/`, and `book-wire/`, plus the three relevant docs. Those original tests have distinct scopes: actual Font12 proof head49a2401e/run42171288439 used official624c; BookId5+anchor5 proof head93424e79/run42187284494 is std-only. Mock Text wire tests are not actual native/LAN tests. Historical whole-source inverse/pin tests target their original candidate and require explicit composition treatment, not silent hash replacement.

The new eight pure tests check both complete inverse transformations, untouched navigation manifest entries and click tail, exact added modules/lazy query and rejection of changed inputs. Reproduce with GUIDE_NAVIGATION_INPUT and GUIDE_CONSUMER_INPUT pointing to directories containing exactly the two named source files, then run test_merge.py. No Cargo, Rust, metadata, planner or native execution is used.

## 41f audit and remaining blockers

Three full official files (books.rs/native_hooks.rs/remote.rs) were fetched at41f, verified against their Git blob IDs, and inverse-patched back to the pinned f4dc SHA-256s. `open_book`, `close_book`, `book_input`, `native_client_packet` and `handle_packet` bodies are byte-identical. The book drawing path now delegates its dark background to the new native cancel/blur hook. The custom TConstruct draw still cancels native draw and uses its own fill; that does not prove support for the new background hook. No complete41f inventory or native proof was run.

The legacy response cancellable HEAD still bypasses native packet observation and fails the mandatory Pluto guard. It must be replaced by a separately reviewed internal bridge while retaining old envelope, sequence, repeated same-sequence replies and overlay behavior. Mixins5a64 builds one original Index before staging support modules and resolves injections against it; another mod cannot target a newly added module function in that same plan (plan.rs322,429-439,477). No two-stage apply or extra RETURN guard exception is authorized by this document.
