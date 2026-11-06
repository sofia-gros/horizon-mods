# Explicit formatter for guarded Windows staging

Source-only follow-up to `117c36092a3c29142507a14035017d0df73ef6d7`. No formatter archive, executable, installer, Windows process, native build or game was downloaded/run for this change. Only the two official public text metadata responses were read. The old successful Windows source CI remains at5e14; this changed PowerShell entry has not yet been parsed on Windows.

## Concrete missing prerequisite

Official Horizon41f's launcher toolchain downloads rustc, Cargo, rust-std and rust-mingw, but not rustfmt. Pluto's unchanged terrain and damage source-regeneration checks require `rustfmt --edition 2024 --emit stdout`. Previously the Windows entry neither checked that dependency nor prevented an unrelated formatter on the ambient search path from being chosen. A missing formatter stops before apply; it is not a reason to bypass the checks.

The official dated Rust1.99.1 channel manifest and the GNU rustfmt archive SHA256 sidecar were independently read over HTTPS. Both identify archive SHA256 `488f4cdcec501721fb2986a4c36c8180d5606f5b569f942fb7c8886f36a7e2aa`. The manifest declares rustfmt-preview1.10.0, commit `58cf8f9c35b355199727627d07ae34a4e2846d67`, and an available x86_64-pc-windows-gnu target. Exact URLs, response sizes and manifest SHA are in the adjacent provenance JSON. The general web reader could not retrieve these URLs; a bounded Python HTTPS text read returned HTTP200. This was not an access-denial workaround or a binary download.

## Narrow runtime change

The explicit `-RustfmtArchive` / `--rustfmt-archive` argument supplies an already obtained official tar.gz. The user-selected existing toolchain must contain the corresponding `bin/rustfmt.exe`. The helper does not install/extract files, fetch anything, discover another toolchain or alter system configuration.

Before any formatter execution, admission checks the entire selected toolchain with the existing seal, the archive's pinned SHA, safe/unique regular archive members and budgets, and byte equality between its exact rustfmt executable member and the selected installed binary. Archive reads are capped at64MiB, expanded listed members at256MiB and512 entries. The verified archive/executable identities and reported version enter the ready-cache input. The full input snapshot rechecks them before ready/startup. The input archive remains disjoint from generated output.

The launcher scrubs inherited `PLUTO_GUARD_RUSTFMT` and passes the verified absolute selected binary path in that variable. The two existing checkers now use only that explicit absolute path when supplied; an explicit relative/empty value rejects. With no variable, prior Linux/manual `rustfmt` behavior remains. Their order checks, ownership/exclusivity, source/function hashes, extraction, hunks, edition/emit flags and exact formatted-result comparisons are unchanged. The inverse source test restores both prior checker files byte-for-byte. No Windows system-directory search or broad shadow-file scan is needed.

The version check compares the manifest's package version/commit with stable formatter output; its printed build date is not guessed from the distribution publication date. The actual executable's output and archive layout are still an execution gate, not observed in these pure tests. Synthetic archive tests patch the checksum only inside test scope and never execute their fake executable bytes.

## Validation and limits

- 13 new pure tests passed: fixed metadata, checksum before parsing, installed-byte equality, missing/duplicate/link/traversal rejection, budgets, parent links, version/commit/channel, environment selection, both explicit/default checker routes, immutable comparison behavior, source inverse and pre-execution/cache ordering.
- The existing20 latest-launch pure tests passed unchanged. No PowerShell parser, rustfmt, Cargo, metadata, native planner or game ran.
- The first inverse regression used the binding-checker SHA for the cell-checker expectation and failed. The expected SHA was corrected to the actual unchanged cell-checker bytes; the two inverse transformations and production routing were not weakened. Final12 passed, then13 passed after adding the old-checker routing admission regression.
- Previously successful helper regeneration used the1.95 toolchain's formatter. **The1.99.1 formatter has not regenerated those helpers.** A later approved remote guard run must execute that exact comparison. A mismatch must be examined as formatting versus semantic drift, never accepted by relaxing the guard or replacing arbitrary body bytes.
- Both new checker files need to be taken together with this launch/helper delta; launcher admission requires their exact reviewed SHA256 before any formatter runs. Missing/older checker routing must not be treated as a completed Windows admission path. This source checkpoint does not change the existing full-plan result or authorize an apply/build.

Official source: [launcher toolchain](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/launcher/src/rust_toolchain.rs). This change does not touch that repository or any fork.
