# Roadmap

Two parts: the long-term plan (owner-defined order) and the **overnight run** (what the agent does unattended, within which limits).
Principles: [PlutoLib design principles](../pluto-lib/docs/design-principles.md). Specification draft: [fabric-api-design.md](../pluto-lib/docs/fabric-api-design.md).

## 1. Long-term plan

| # | Step | Done when |
|---|---|---|
| 1 | **Fix the PlutoLib specification.** Fabric-API-like modules; independent mod; per-product releases; Tinkers-needed scope first. | Spec approved by the owner and merged. |
| 2 | **Find the APIs PlutoLib needs** from `main` hooks and open PRs ([hook map](../pluto-lib/docs/fabric-api-hook-map.md)), checked against the original Tinkers' Construct source for shape. | Every legacy hook and every API the open PRs add is mapped to a PlutoLib module or marked mod-specific. |
| 3 | **Build the APIs**, slice by slice (module, tests, one Tinkers group moved onto it, local smoke). Port PlutoLib to the current Horizon. | PlutoLib builds as its own mod on the pinned Horizon and covers the modules of step 2 in the agreed order. |
| 4 | **Move today's Tinkers `main` onto PlutoLib**, removing direct injections. | `tools/forbidden_api_baseline.json` is empty for `tconstruct/`; CI and smoke green. |
| 5 | **Rewrite the open PRs onto PlutoLib while merging them** (order in `docs/migration/pr-classification.md`). | Each PR: CI green and smoke; originals closed with a pointer. |
| 6 | **Clear every PR and issue.** | Each is merged, closed as superseded, or deferred with a reason. |
| 7 | **Release.** | Release criteria below. |
| 8 | **Inventory what the original Tinkers' Construct has and `main` lacks.** | A tracked list by feature group. |
| 9 | **Deliver the rest as updates** on PlutoLib. | One release per feature batch. |

Rules that apply throughout: slices follow **Fabric API modules**, never one Tinkers feature (no API exists for a single feature such as the Part Builder);
a Horizon/Mixins limit never removes an API (it ships as a documented stub); no new features and no new direct injections while PlutoLib is built;
tags are `<directory>-v<semver>` per product (`pluto-lib-v1.0.0`, `tconstruct-v1.1.0`), and the release check requires the tag to equal the product's `[mod] version`.

Release criteria (step 7): CI green on the release commit; `tools/smoke-local` passes against the pinned Horizon; the release is built from a written-down PlutoLib/Horizon/Mixins triple; save/reload and LAN paths of the shipped feature groups are exercised, not assumed; README and docs state what is and is not supported.

### 1.1 Port by domain (steps 3 and 4 in the order the owner approved)

The original Java at the pinned revision is the specification (the existing Rust is of unknown correctness). Each domain is one design, one PlutoLib API set, one build and one in-game check, and its direct injections leave Tinkers. Out of scope: everything for other mods (the original's `plugin` package: no integration items, blocks or systems). Rules: PlutoLib keeps its responsibilities (design principles); anything Horizon does not offer is a PlutoLib injection plus a documented API; mods on PlutoLib never compete for injections (PlutoLib owns each injection point once and dispatches to callbacks; a mod ships none).

| Phase | Domain (original size) | PlutoLib foundations it needs first | PRs |
|---|---|---|---|
| A | **Foundations**: creative tabs (paged, stacks with data that can be taken out; the feature of the old bundle, redesigned), block data and ticking per position, fluid tanks, menus (screens), rendering base, injection-ownership check | those themselves | 6 to 8 |
| B | fluids (2k lines) | fluids | 1 to 2 |
| C | gadgets (3k) | item use, entities | 1 to 2 |
| D | world: slime islands, plants, saplings, worldgen (6k) | block behaviour, worldgen | 2 to 3 |
| E | shared: blocks, items, recipes, commands, config (6k) | commands, config, data | 1 to 2 |
| F | common: network, tags, data loading (10k) | networking, data | 2 to 3 |
| G | tables: tool stations, Part Builder, anvil, book (10k) | menus, block data, rendering, creative tabs | 3 to 4 |
| H | smeltery: smeltery, melter, casting, faucet, channel, tanks (20k) | fluid tanks, block data, multiblock structures | 4 to 6 |
| I | tools: tools, materials, traits, mining, melee, ranged, armour (38k and part of library) | item use, entities, attributes | 5 to 8 |
| J | finish: whole-game check, the open PRs and issues, the gap list against the original | | 2 to 3 |

Each domain ends with: a table comparing the original's behaviour with the port (in the pull request), the domain's direct injections removed, the in-game check written down as run or not run, and PlutoLib's pages and tests updated. Phase status is kept in the decision log.

## 2. Overnight run (unattended)

The agent keeps working without stopping until the owner is back. It decides for itself inside the limits below, records each non-obvious decision in the log at the end of this file, and never waits for an answer it can replace with the conservative default.

### 2.1 Order of work

Each item is finished (PR, local gate green, merged) or parked (see §2.4) before the next starts. Up to three items may be in flight; at most five of the agent's PRs are open.

| Order | Item | Exit criterion |
|---|---|---|
| N1 | Merge the PlutoLib specification PR (#952) when CI is green. | Spec on `main`. |
| N2 | **Prove the dependency mechanism.** Build two experimental bundles (an `[api]` PlutoLib-shaped mod and a mod that declares `pluto_lib = "^1"`) with the launcher's `native-build --server-output`; run the result if possible. Record the outcome in the spec (works / what must change). | Spec states the verified mechanism, or the exact failure and the fallback. |
| N3 | **Docs track D1:** find every test, CI step and tool that reads a document, so deleting documents cannot break them (§3). | List in the PR; tests that guard prose identified. |
| N4 | **Slice 1: foundation and packaging.** `Event<T>`, event phases, `Identifier`, `ActionResult`, support matrix; PlutoLib restructured as a trait mod with an `[api]` package; Tinkers made a trait mod depending on it; per-product release workflows (not triggered, tags are never pushed overnight). | PlutoLib builds as its own bundle; Tinkers builds against it; `tools/smoke-local` green; CI green; merged. |
| N5 | **Docs track D2: delete all existing documents** (archive tag keeps them) and fix the tests that read them. | Local tests green, merged. |
| N5t | **Test track T1: prune the test code** to what is necessary (§4). Measure before and after (count, lines, run time, size). | Policy applied, local test run well under two minutes, merged. |
| N6 | **Slice 2: modules on native hook points** (`resource_loader_v0` and `loot_api_v2`, `item_api_v1`, `events_interaction_v0`, `object_builder_api_v1` block behaviour). Each module: API, tests, support-matrix entry, one Tinkers group moved onto it with its direct injections removed, lint baseline lowered, smoke green. Stubs for anything the pinned Mixins rejects. | As many modules as time allows, each merged separately. |
| N7 | **Docs track D3: write the new documents from scratch** (§3). | Merged. |
| N8 | Re-run the classification of open PRs against the new `main` and update `docs/migration/pr-classification.md`; take A/B PRs that now apply cleanly through the gate. Close an original only after its content is on `main`. | Updated plan merged; batches merged. |

If an item is parked, the agent continues with the next one it can do safely and returns later.

**When N1 to N8 are done or parked, the agent does not stop.** It carries on with the long-term plan (§1) in order, from the first step that is not finished (steps 3 to 9), cutting each step into slices and pull requests under the same permissions, defaults, gate policy and stop rules, until the owner is back. Release (step 7) happens only when its criteria are met, never to fill time. If a long-term step needs a decision the owner has not delegated (§2.3), the step is parked and the next one is taken.

### 2.2 Permissions granted for the run (given by the owner)

Allowed, without asking:

1. Create branches and push them. Force-push is allowed when it is the right tool, but prefer a merge or a new commit; for `main` only after an archive tag and a decision-log entry.
2. Open, update and **merge its own pull requests** once the local gate passed (§2.6); CI is a follow-up check, not a precondition.
3. **Close an original PR with a pointer comment** after its content landed on `main`.
4. Edit, move and **delete any file in this repository, documents included** (§3), through pull requests; an archive tag keeps the old tree.
5. Install development tools with `mise`; run long builds and tests on this PC at normal priority; keep at least 10 GB free on `A:` by cleaning the agent's own caches under `.local/`.
6. Run the Horizon launcher's `native-build --trust-native-code` on bundles written in this repository or under `.local/` only.
7. **Push release tags and create GitHub releases**, but only when the release criteria of step 7 are met; a release is never made to "have something".
8. **Create, comment on, label and close issues and pull requests** to resolve them (the agent is responsible for clearing them): close with an explanation, never silently, and link the PR that resolved it.
9. **Change repository settings** (for example branch protection, required checks, default branch behaviour, Actions permissions) only when the agent judges it indispensable for the work; record each change, the reason and how to undo it in the decision log.

Not allowed: touching the old fork `/a/Project/horizon` or any other repository; acting outside this repository, its GitHub repository and `.local/`; spending money; account-level changes (profile, billing, other repositories' access, personal tokens, SSO).

### 2.3 Defaults for decisions the owner would otherwise make

- Prefer the option that keeps history and can be reverted (a PR over a direct push, a stub over a missing API, an archive tag over deletion). Irreversible actions (releases, closing issues in bulk, settings) need a written reason in the decision log.
- API shape: follow Fabric API; where Rust forces a difference, use the idiomatic Rust form and note it.
- Names (modules, crates): the spec's names; if wrong, rename in a later slice rather than blocking.
- Language of documents: user-facing README and play guide in English with a Japanese edition; developer documents in English; the agent file (`AGENTS.md`) in English.
- Unknown host behaviour: measure it (build, run, smoke) before documenting it; write "unverified" otherwise.

### 2.4 Stop and park rules

- **Same failure three times** (CI or local), or a fix that would need an owner decision outside §2.3: park the branch (leave the PR open as a draft with a comment saying why) and move on.
- **Disk below 10 GB free on `A:`** after cleaning caches: stop starting builds; docs-only work continues.
- **Anything listed under §2.2 "Not allowed"**: skip it and list it in the morning report.
- **Do not claim "verified" without running it.** Every PR states what was run and what was not.

### 2.6 Gate policy: local first, CI second

CI is slow (about 25 minutes). Whatever can be verified on the owner's PC is verified there and does not wait for CI:
`python tools/lint_forbidden_api.py`, `python -m unittest discover -s tests`, the Python tests the change touches, `tools/smoke-local/build.py`
and `smoke.py`, cargo checks. A pull request is merged when this local gate passed and its description says exactly what was run. CI then
runs on `main` and on the PR as a second, independent check; a CI failure after merge is fixed forward with the next commit. Where CI
cannot add information (documentation only) it is skipped. Work to make CI faster or lighter (smaller job, cached builds, path filters) is
part of the plan and is done when it unblocks something.

### 2.5 Morning report

Whenever a turn ends and when the owner returns: one message listing, per item, merged / parked / not started, the PR numbers, the local gate results, each decision taken from §2.3, and the questions that still need the owner. The decision log below carries the same facts for the repository.

### Decision log

| Date | Decision | Why |
|---|---|---|
| 2026-11-08 | Tag scheme `<directory>-v<semver>` (hyphen, not slash). | Unambiguous release URLs and non-overlapping trigger globs. |
| 2026-11-08 | Spec module names stay as drafted. | Owner delegated; refine per slice. |
| 2026-11-08 | N2 done: `[api]` + `[dependencies]` works with the official launcher (two experimental bundles linked). Slice 1 adopts it. Our Mixins-CLI pipeline stays as a fast development gate; the launcher's `native-build` becomes the authoritative build for PlutoLib and Tinkers. | Measured, see the spec. |
| 2026-11-08 | If the overnight items finish before the owner wakes, continue with the long-term plan (§1) in order instead of stopping. | Owner instruction. |
| 2026-11-08 | Owner widened the run: all existing documents may be deleted and rewritten; local verification replaces waiting for CI; release tags, releases, force-push, issues and repository settings are allowed (settings only if indispensable). | Owner instruction at roadmap review. |
| 2026-11-08 | N1 to N8 status: N1, N2, N3, N5, N5t (first pass), N7, N8 done; N4 done except making Tinkers a PlutoLib dependant; N6 started (first four modules merged). Continuing with the long-term plan. | Progress record. |
| 2026-11-08 | PlutoLib events are fed from Horizon's native hook points (`native-hooks.json`, each annotated with its NeoForge event) through one file, `hooks.rs`; Fabric members the host cannot serve are stubs in the support matrix. | Stable surface; less Mixins churn. |
| 2026-11-08 | Windows CRLF problem fixed at the root: `.gitattributes` forces LF, `check_line_endings.py`, and a lint rule for Python `write_text` without `newline=`. | Owner reported four repeats. |
| 2026-11-08 | The official launcher drops a whole mod when Mixins refuses any hook, and Mixins breaks `0. ..=1.` in some Horizon files. The shipped Tinkers package is therefore built from the slice (`package_hnmod.py`) and also leaves out every injection that edits those files (`launcher_unsafe_files` in `slice.json`: slime islands, trees, saplings, 17 injections) until Mixins is fixed. | Measured with `native-build`; otherwise users get no Tinkers at all. |
| 2026-11-08 | Old open PRs: the 250 left are classified (A 9, B 189, C 50, X 2); the A ones are stacked on unmerged B PRs and wait with their chains; B and C are rewritten onto PlutoLib after the modules exist. #832 and #840 were already part of main and are closed. | Roadmap step 5 ordering. |
| 2026-11-08 | Mixins time is set by the number of `[[state]]` blocks (about 7 s each in `plan` and again in `apply`), not by injections or source size: 1 injection 61 s, 15 injections with 69 states 525 s, all 290 with 108 states 894 s. Tinkers' 108 states were folded into one state whose type holds every field (`tconstruct-state.rs`), the old accessor names kept as thin methods. `plan` 894 s to 29 s; stage + cargo check 1906 s to 211 s; launcher full build about 20 min to 7 min. A new `[[state]]` must not be added per field: add a field to the struct. | Measured with the pinned Mixins release build on the owner's PC (Ryzen 7 2700). |
| 2026-11-08 | T1b: 107 Tinkers test files deleted (data and source-text comparisons that run nothing) and 10 manifest-string methods removed from 9 files that run real code. Python suite 247 tests / 370 s to 26 tests / 76 s; tracked test files 591 to 484, lines 28.6k to 23.9k. The owner's instruction: remove tests that cannot be trusted. | Policy of section 4 applied; what remains executes helper code or tools. |
| 2026-11-08 | Tinkers is ported by domain, from the original Java. The owner: the open-source Java is the truth, the current Rust is unverified, per-injection PRs and builds would be a thousand cycles. Units: tool stations, tool behaviour, slime world, casting, smeltery, networking/config/data, book, rendering (about 15 to 25 PRs). Anything Horizon does not offer becomes a PlutoLib injection plus a documented API. | Fewer, larger cycles; correctness is judged against the original. |
| 2026-11-08 | Owner's rules for the Tinkers port: mod-integration items, blocks and systems are not written; PlutoLib's responsibilities and spec are kept; the old creative-tab paging and take-out feature stays in the new PlutoLib; mods on PlutoLib must not compete for injections (PlutoLib owns each injection point once, mods only register callbacks). Phases: A foundations (block data, fluid tanks, menus, rendering, creative tabs), B fluids, C gadgets, D world, E shared, F common, G tables, H smeltery, I tools, J finish; about 25 to 40 PRs. | Owner's approval of the roadmap. |
| 2026-11-08 | Phase A (foundations) done: creative tabs (#976), injection-ownership check (#977), block entities and NBT (#978), item slots and fluid tanks (#979), screens and menus (#980), block entity drawing (#981). Correction of an earlier estimate: Horizon already offers block entities, containers, menus, screens, fluid tanks and drawing to native mods, so these are Fabric-shaped wrappers of its hook points, not new injections; only the creative tabs needed a PlutoLib injection. Each has an in-game probe line; what the screens and drawing look like, and clicks, the server button, fluid transfers between blocks and save/reload are still unverified. | Phase A of section 1.1. |
| 2026-11-08 | Phase B (fluids) first part done (#983): PlutoLib fluid definitions and translations (a PlutoLib module of its own, as the owner allowed), Tinkers' 42 own fluids defined from a table generated from the original `TinkerFluids.java`; compat fluids for other mods not ported. Not done: the items of the package (bottles, the potion bucket, food containers) and the in-world blocks and buckets of the fluids, because Horizon has neither for a mod's fluid (stubs in the support matrix). Docs: all API pages are now in the Vue guide style (#984). Next: phase C needs custom entities (shuriken, glow ball, fancy frames) and capabilities; Horizon's entity support for native mods is to be checked first. | Phase B of section 1.1. |
| 2026-11-08 | Phase C (gadgets) findings, from reading the original `gadgets` package against Horizon: the throwballs (glow ball, EFLN ball, quartz and flint shuriken) are `@Deprecated` in the original (the shurikens are tool ammo in the tools domain); the thrown entity types can be declared as content (template `minecraft:snowball`, an item, `damage`) with behaviour through `ProjectileEvents` and `World` (merged in #986). The fancy item frames and armor stands copy entity templates Horizon does not offer (only snowball, villager, wandering trader, zombie). The punji block needs state properties of its own (supported: `state_properties`), per-facing shapes, `can_survive`, placement and shape-update logic, and entity contact damage; the cakes need food and effect helpers on the player; the piggy backpack needs a capability and riding. So the remaining gadgets are limited by Horizon's content model and by player helpers, not by PlutoLib. Next in this phase, in order of how much Horizon allows: the punji block through the block-behaviour hooks, then cakes (needs a player effects and food API), then the throwballs. Frames and armor stands wait for Horizon entity templates (a stub in the support matrix). | Phase C of section 1.1. |
| 2026-11-08 | Phase C reordered after the player effects and food API (#988) and a closer look at the cakes and the punji. The cakes' effects (bouncy, double jump, antigravity, returning) are Tinkers' own mob effects, which are not ported yet (they belong with the common and tools domains), so only the magma and blood cakes could work now, and Horizon cannot set a player's food (stub). The punji and the world plants of phase D both need block behaviour: Horizon has the hooks (`can_survive`, `state_for_placement`, `update_shape`, `block_entity_inside`, `may_place_on`) but they speak `BlockStateId`, and only `state_with_property` turns one into another; there is no call that names a state's block or reads a property from a hook. So PlutoLib gets a block-behaviour API (`block_api_v1`) first, built on a table of every state of each registered block (built from the declared property values), then the punji, then phase D. The cakes follow the mob effects. Done in the same slice: `block_api_v1` (can_survive, state_for_placement, update_shape, entity_inside; no injections, only Horizon's hooks) and the punji ported on it: stuck to any of six sides, connecting to punji of the same side, refused or broken without support, checked in game. Differences from the original: it takes damage from a cactus-style contact (1 per tick, no extra for falling distance and not only in the sticks' half), it cannot be waterlogged, and its tooltip is not shown yet. | Phase C and D of section 1.1. |
| 2026-11-08 | Phase C closed for now. Done: the punji (#989). Parked, with the reason: the throwballs (deprecated in the original, they place the glow block of the common domain and are replaced by the shuriken of the tools domain, so they come with phases E and I), the cakes (wait for Tinkers' mob effects and for a way to set a player's food), the fancy frames and armor stands (Horizon has no entity templates for them), the piggy backpack (needs riding and a capability). Next: phase D (world), starting with what its plants and saplings need from block behaviour and worldgen. | Phase C of section 1.1. |
| 2026-11-08 | Phase E content gap found by comparing the original's language keys with the content files: 133 blocks and 311 items of the original have no content file (the ids may come from elsewhere; not yet checked one by one). First part done: the 16 wood signs (standing, wall, hanging, wall hanging for bloodshroom, enderbark, greenheart and skyroot) with items, loot, recipes and tags, using Horizon's sign support. The sign editor uses a vanilla wood's background (crimson, warped, jungle, birch) because the original has no 24x26 editor textures. Placing all four forms was checked in game; typing text and the look were not. The owner's release scheme is in the memory: 0.0.1 when this roadmap is done, 0.5.0 mostly playable, 1.0.0 complete; then a new roadmap. | Phase E of section 1.1. |
| 2026-11-08 | Phase E: the 11 platforms (iron, gold, cobalt, four copper stages, four waxed) are blocks now (the old port had gold as an item placeholder). Shapes are the original's 64 combinations as content `shapes`; connection above and below (same family, or a sturdy block below) is PlutoLib block behaviour, checked in game. Not done, with the reason in the support matrix: copper never weathers (random ticks can only change properties, not the block), they do not connect sideways to ladders, bars and the like (hooks cannot name or tag a neighbour), and they cannot be waterlogged. | Phase E of section 1.1. |
| 2026-11-08 | Phase E: the 10 potted slime ferns and saplings are blocks (flower pot templates, the original's models and textures, loot of a pot and the plant); placing two was checked in game. Still missing from the gap list and parked with their domains: the gauges, channels, cannons, alloyer and proxy tank (smeltery, phase H), the tool forge and cast chest (tables, phase G), the mob heads (need skull support in Horizon), the fluid blocks and buckets (stubs in the fluid API) and the cakes and spawn eggs. | Phase E of section 1.1. |
| 2026-11-08 | Phase F (data): the content had 119 of the original's 803 tags. Now 696 more are added and 7 completed from the original's generated tags (blocks, items, fluids and entity types of tconstruct, forge and minecraft), with the tags and entries of other mods left out (no integration) and every non-vanilla entry marked optional, so an id that does not exist yet cannot fail a tag. The game loads with them (probe passes, no tag errors in the log); that the tags do what the original's code expects (smeltery structures, harvesting, casts) is not checked and belongs to the domains that use them. | Phase F of section 1.1. |

## 3. Documentation track

Problem: hundreds of markdown files were written per implementation unit (about 100 under `pluto-lib/docs`, about 260 under `tconstruct/docs`, plus
validation receipts and reference dumps), old and new mixed, not readable as a project. The owner's decision: **every existing document may go; start fresh.**
The new set is small, current and written for people; per-feature detail lives in code (rustdoc, tests), not in prose.

| Step | What |
|---|---|
| D1 | Find every test, CI step, tool and package script that reads a document (so removal cannot break them). |
| D2 | Tag `archive/pre-docs-cleanup`, then delete all existing `.md`, receipts and reference dumps. **Kept: `docs/branding/` (the banner and the icon).** Fix or remove the tests that guarded prose. |
| D3 | Write from scratch: `README.md` (what it is, status, how to run; English and Japanese; **uses the existing banner and icon from `docs/branding/`**, and in-game screenshots captured with `horizon-ctl screenshot` from the local smoke run and stored under `docs/images/`); `docs/architecture.md`; `docs/development.md` (build, local gate, CI); `docs/releasing.md`; `docs/roadmap.md`; `docs/migration/` (plan and PR classification, regenerated); `pluto-lib/README.md` and `pluto-lib/docs/` (design principles, specification, support matrix, generated hook map); `tconstruct/README.md` with one play guide; `AGENTS.md` / `CLAUDE.md`; API reference generated by rustdoc. |
| D4 | Keep it small: a CI/local check that fails when new per-implementation markdown appears (a count/size budget for the documentation directories). |

## 4. Test policy

The existing tests were written in bulk, including tests that exist only to pass: they assert source text, copy of pinned upstream files or document wording, so they do not
guarantee quality and they cost build time and repository size. The owner's decision: **keep only the tests that are necessary.**

Keep:
- tests of real logic with branches that can go wrong (parsers, formulas, transactions, conversions, release validation, the lint);
- tests that start the real game and check what a player would see (`tools/smoke-local`);
- a test only if it would fail when the behaviour it names is broken.

Remove:
- tests that read a source or document file and assert it contains a string, a count, or an ordering;
- copies of upstream source or reference dumps kept only to be compared with;
- per-feature regression tests of trivial data (a list of ids, a count of files);
- fixtures that duplicate host code, and anything else that only mirrors the implementation.

Rules for new tests: written with the change; one test per behaviour; check once that it fails when the code is broken (temporarily break it); never added to satisfy CI or a count.
Budget: the local unit-test run stays under two minutes; test code and fixtures stay a small fraction of the repository; both numbers are written in `docs/development.md` and checked.
Measure first (count, lines, run time, bytes of tests and fixtures), then prune, then record the after-numbers in the pull request.
