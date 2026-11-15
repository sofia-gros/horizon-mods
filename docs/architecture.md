# Architecture

## Dependency boundary

```text
Tinkers gameplay/data -> PlutoLib public API -> PlutoLib host adapters -> official Horizon
Horizon events       -> PlutoLib dispatch   -> registered Tinkers callbacks
```

Only PlutoLib owns injections. It supplies general capabilities and defined callback ordering, not Tinkers identifiers or gameplay rules. Tinkers owns original behavior and content. The official upstream is read-only. Preserve creative-tab paging and taking out stacks. Missing API support is implemented in PlutoLib before the dependent feature is declared complete.

## Active and frozen trees

- `tconstruct/`: clean product, initially an empty API entry point. No direct injections or legacy fallback.
- `pluto-lib/`: existing candidate implementation, NOT automatically certified by this reconstruction. Reuse and audit it; do not rewrite it wholesale.
- `legacy/2026-11-14/`: frozen previous Tinkers working tree and original planning documents. Read-only reference, never an active dependency or package input.
- `tools/port.py`: clean-product boundary, task selection and development packaging.
- `docs/port-plan.json`: small dependency queue; `docs/roadmap.md`: implementation/verification policy.
- Existing root tools/tests/CI: retained, not disabled. Their old-product assumptions must be reconciled in R1. Do not run an old smoke/launcher route as evidence for the new product.

The native manifest declares the PlutoLib bundle dependency. The official launcher supplies that Cargo edge. Its mandatory `horizon-client` Cargo declaration is an owner-approved metadata-only exception, pinned with defaults disabled; Tinkers Rust may still call only PlutoLib. The new linkage requires native validation. A host-independent substitute cannot establish that linkage.

## Completion boundary

For each feature trace registration, real caller, state/data ownership, observable behavior and persistence/sync. Pure tests prove only the exercised rules; compilation proves only compilation. No original gameplay has yet been imported into the clean entry point. Original Java is the specification, not the archived Rust or its documentation.
