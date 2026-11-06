# Font generation fixture: first attempt stopped

Exact source `a4ebdab16b23688b82e6dce19054a1121298f6eb`; [run 42218194849](https://github.com/sofia-gros/horizon-mods/actions/runs/42218194849), job `127893380563`, ended in failure. The original false receipt is preserved verbatim as a JSON value in `result-a4eb.json`.

All 41 pure checks passed (20 runner, 15 preparer, six source). Setup, official f4dc checkout, fixture preparation, and offline Cargo metadata completed. Metadata exited zero with 2,288,267 stdout bytes and no stderr. The next ownership check rejected `wrong fixture workspace`. No actual Rust compilation, 21-name inventory, or six Font tests ran. Final verification is false; this is not a partial native pass.

The generated manifest has an empty `[workspace]` and an internal `inventory-api` path dependency, while verification expects only the fixture root as a member. Cargo's automatic inclusion of in-tree path dependencies provides a concrete source-level explanation. The actual metadata member list was not printed or retrieved, so its exact contents remain unobserved. A correction must retain an exact workspace contract rather than allow arbitrary members.

Full compact console: 19812 UTF-8 bytes, SHA-256 `4fde43bd86e48bf638c9e6b261ae6764cb9a5fc8013d302ea3436ed998e44924`. Metadata stdout SHA-256 `b54e60977ed9a4a03b0431443480f3989187da789549daf5f766ddd804ebd24d`; raw metadata is not included. Minimum reported free space was 89,955,147,776 bytes.

The remote slot was returned at terminal. There is no automatic retry. Existing consumer/runtime bytes, earlier Guide ownership proof, ordinary manifests, and LAN remain unchanged. Game, GPU, live drawing, retained screen authority, language refresh, LAN behavior, and installed package compatibility are outside this attempt.
