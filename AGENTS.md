# Working agreement

## Active direction — 2026-11-14

The owner requested a clean PlutoLib-only Tinkers port, preservation of old files, minimal useful tests/documentation, and continuous work without routine approval pauses. This file and [the active roadmap](docs/roadmap.md) supersede the archived workflow. Historical inventories are reference material, not task selectors or evidence of completion.

## Non-negotiable boundaries

- Original Java at `tconstruct/compatibility.json` is the specification; verify the checkout revision. Original Java and reference checkouts of official Horizon/Mixins remain read-only. Owner clarification (2026-11-14): Mixins support is not a ceiling on PlutoLib. When necessary, PlutoLib directly rewrites Horizon build sources through repository-owned, reproducible patches; do not wait for upstream support. Keep official reference inputs separate from patched build inputs and verify both. No other-mod integration.
- Owner decision (2026-11-14): use the official Horizon and Mixins default-branch HEADs resolved at the start of the current compatibility unit, then record and hold their full SHAs for implementation/verification. Do not substitute an older cached checkout or move pins during validation; an update invalidates affected compatibility evidence. Updating pins is not authorization to bypass native guards.
- `tconstruct/` is the ONLY active Tinkers product. Its Rust code depends on PlutoLib's public API, never Horizon APIs/types, direct injections, host patches, private adapters or legacy files. Owner-approved exception (2026-11-14): Cargo must declare the official launcher's required `horizon-client` dependency as metadata only, pinned to the selected Horizon revision with default features disabled; this permits no direct use from Tinkers Rust code. Zero direct injections, with no grandfathered exceptions.
- `pluto-lib/` owns all host attachment. APIs are general-purpose and Fabric-API-shaped, never named or branched around Tinkers content. Preserve creative-tab paging and taking stacks out. Mods do not compete for injection points.
- `legacy/2026-11-14/` is a frozen working-tree snapshot, including previously uncommitted changes. Never execute, import, package, edit or delete it as a shortcut. Read it only to recover reviewed assets/algorithms; verify them against Java before reuse. Keep licence notices.
- A stub is not usable support for a required feature. Never complete a feature on the strength of a helper, decoder, mock or successful build alone.
- Owner decision (2026-11-14): all previous Phase completion/order restrictions are retired. Isolated official-source native builds and game validation may proceed for the current roadmap unit without another Phase approval. The owner explicitly renewed isolated rebuild permission, including while asleep, on 2026-11-14; do not ask again for routine rebuilds within this scope. Preserve source-integrity, injection-ownership and concrete attachment-contract checks; missing fingerprints require inspection of the actual pinned source/plan, never synthetic approval. Use isolated homes/worlds, never user saves.

## Fixed porting strategy — owner decision, 2026-11-14

- The first implementation unit is **Tinkers common foundations + tool construction/use**, not slime dirt or individual block behaviors. Establish the original-wide capability/dependency map (R3) and execution path (R1) before starting that unit (R2).
- Cover registration/data loading, materials/parts/tool definitions and calculations, stack state, required crafting stations/inventories/screens, use/durability/repair, required modifier paths, persistence/synchronization and presentation as one coherent domain. Derive precise coverage from Java; do not silently replace it with one demonstration tool or omit unsupported behavior.
- Inspect later fluids/casting/entities/world consumers before fixing shared API contracts. Build only the general PlutoLib capabilities actually required by the domain, with real host adapters; keep Tinkers rules in Tinkers. Do not attempt a speculative complete API platform first.
- Small linkage probes are infrastructure only. Internal subtasks are allowed, but helpers, individual injections or isolated blocks are not independent completion/review/build milestones. Integrate and review the domain, then batch authorized native/game validation; retain inexpensive focused checks during implementation.
- Verify the complete loop: material -> part -> tool -> use/durability -> repair -> save/reload, plus relevant state synchronization and presentation. Slime soil belongs to the later world/vegetation domain, not the first port.
- Do not change this strategy or return to the archived per-injection workflow on an agent's own initiative. A strategy change requires the owner's explicit decision; ordinary dependency ordering and implementation choices do not require approval. Keep this section, the roadmap and machine queue consistent when an approved change occurs.

## How to work

1. Run `python tools/port.py next`; read the selected task and the relevant original sources. Keep one active integration unit. The machine queue is `docs/port-plan.json`.
2. Trace original entry -> required state/data -> public PlutoLib API -> actual host caller -> observable result BEFORE expanding helper implementations.
3. Missing general API: implement it in PlutoLib first, including its actual host adapter, then finish the Tinkers consumer. Do not ask the user about ordinary API names, file choices, or whether to continue.
4. Prefer the existing validated implementation when source-correct. Do not copy whole legacy directories, carry injection fallbacks, or rewrite correct code merely for cleanliness.
5. Test only behavior that could be wrong. Use source-derived expectations and the production path. No prose/source-string assertions, trivial count padding, duplicate host implementations or mass test generation. Boundary/package validators may inspect real build inputs and archives.
6. During edits run targeted checks; at an integration-unit boundary run related regressions once. Batch expensive validation and reuse identical binaries. Never run every suite after every small edit.
7. Update only the current task's short result, blockers and evidence. Public API changes need a concise usable example/caveat, not a new report. No per-iteration diaries or test-count progress claims.
8. Continue immediately to the next unblocked step in the SAME turn. Do not stop to ask “continue?”, “implement the API?”, “update docs?”, or “run the planned checks?”. Do not use a timer as an artificial gap. While a background job runs, do independent authorized work; do not poll harness-tracked jobs.

## Real blockers, not habitual pauses

A failed check is a diagnosis task, not an approval question. Do not repeat an unchanged failing command. When a route fails, inspect the cause, try a materially different permitted route, and record the unresolved prerequisite if none is supported. Missing host hooks alone do not prove impossibility.

Stop only the affected task for: explicit permission denial; an irreversible/outward-facing action without authorization; risk to user saves or unrelated changes; credentials/cost/licence acceptance; genuinely conflicting requirements; or no safe feasible route. Continue independent queued work. Ask one consolidated question only when the answer is actually required to proceed. Never fabricate approval or relax a test/guard to avoid a question.

Owner authorization (2026-11-14): commits, pushes, PR creation and merges needed for this work may proceed without renewed confirmation, including while the owner sleeps. Preserve unrelated/uncommitted changes and the archive; this does not authorize destructive resets, history rewriting, releases or account changes. Keep upstream reference checkouts read-only; PlutoLib-owned direct changes belong in isolated Horizon build copies. Use LF. Owner storage rule: work and build caches stay on A:; do not use D:. Maintain at least 50 GB free on A:, including during builds; reserve additional build headroom before large builds and keep user data and validation evidence intact during cache cleanup. No new agents/sessions unless permitted by the active conversation/harness.

## Completion and honesty

`python tools/port.py check` checks the clean product boundary, not semantic parity. `python tools/port.py package --output PATH` packages only that checked product. Existing CI and historical tools/tests remain present; do not disable them to conceal migration failures. Reconcile their entry points in task R1.

Report in Japanese: what now works, what remains, and what actually ran. A new empty entry point is infrastructure, not a Tinkers feature. Source-ready and game-verified are distinct; evidence must identify the input revision/hash and scenario. Changed inputs invalidate the corresponding evidence. Planning rules cannot guarantee AI obedience or complete parity: enforce structural constraints in tooling, review semantics against Java, and observe gameplay.
