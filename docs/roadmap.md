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

## 2. Overnight run (unattended)

The agent keeps working without stopping until the owner is back. It decides for itself inside the limits below, records each non-obvious decision in the log at the end of this file, and never waits for an answer it can replace with the conservative default.

### 2.1 Order of work

Each item is finished (PR, CI green, merged) or parked (see §2.4) before the next starts. Up to three items may be in flight; at most five of the agent's PRs are open.

| Order | Item | Exit criterion |
|---|---|---|
| N1 | Merge the PlutoLib specification PR (#952) when CI is green. | Spec on `main`. |
| N2 | **Prove the dependency mechanism.** Build two experimental bundles (an `[api]` PlutoLib-shaped mod and a mod that declares `pluto_lib = "^1"`) with the launcher's `native-build --server-output`; run the result if possible. Record the outcome in the spec (works / what must change). | Spec states the verified mechanism, or the exact failure and the fallback. |
| N3 | **Docs track D1: inventory and policy** (see §3), as a PR containing only the plan and tooling. | Plan merged; nothing deleted yet. |
| N4 | **Slice 1: foundation and packaging.** `Event<T>`, event phases, `Identifier`, `ActionResult`, support matrix; PlutoLib restructured as a trait mod with an `[api]` package; Tinkers made a trait mod depending on it; per-product release workflows (not triggered, tags are never pushed overnight). | PlutoLib builds as its own bundle; Tinkers builds against it; `tools/smoke-local` green; CI green; merged. |
| N5 | **Docs track D2: remove obsolete and per-implementation documents** as defined in §3 (history kept by an archive tag). | CI green (tests that read documents updated), merged. |
| N6 | **Slice 2: modules on native hook points** (`resource_loader_v0` and `loot_api_v2`, `item_api_v1`, `events_interaction_v0`, `object_builder_api_v1` block behaviour). Each module: API, tests, support-matrix entry, one Tinkers group moved onto it with its direct injections removed, lint baseline lowered, smoke green. Stubs for anything the pinned Mixins rejects. | As many modules as time allows, each merged separately. |
| N7 | **Docs track D3: rewrite** README and the kept guides (§3). | Merged. |
| N8 | Re-run the classification of open PRs against the new `main` and update `docs/migration/pr-classification.md`; take A/B PRs that now apply cleanly through the gate. Close an original only after its content is on `main`. | Updated plan merged; batches merged. |

If an item is parked, the agent continues with the next one it can do safely and returns later. If everything is parked or done, it writes the morning report and stops.

### 2.2 Permissions granted for the run (given by the owner at roadmap approval)

Allowed, without asking:

1. Create branches and push them (never force-push; never rewrite history of `main` or of a branch someone else uses).
2. Open, update and **merge its own pull requests** when CI is green and the local gate passed (merge commits, as before).
3. **Close an original PR with a pointer comment** after its content landed on `main`. Never delete a PR, issue or branch of the owner's.
4. Edit, move and delete files in this repository through pull requests (documents included, §3); an archive tag `archive/pre-docs-cleanup` keeps the old tree.
5. Install development tools with `mise` and run long builds/tests on this PC at normal priority; keep at least 10 GB free on `A:` by cleaning the agent's own caches under `.local/`.
6. Run the Horizon launcher's `native-build --trust-native-code` on bundles written in this repository or under `.local/` only.
7. Comment on the agent's own PRs.

Not allowed (these wait for the owner): pushing release tags or creating GitHub releases; force-pushes; deleting or editing the owner's other repositories and `/a/Project/horizon` (the old fork); changing repository settings, secrets, branch protection or GitHub permissions; opening or closing **issues** (triage lists them in a report only); any spending or account action; touching anything outside this repository and `.local/`.

### 2.3 Defaults for decisions the owner would otherwise make

- Prefer the option that keeps history and can be reverted (a PR over a direct push, a stub over a missing API, an archive tag over deletion).
- API shape: follow Fabric API; where Rust forces a difference, use the idiomatic Rust form and note it.
- Names (modules, crates): the spec's names; if wrong, rename in a later slice rather than blocking.
- Language of documents: user-facing README and play guides in English with a Japanese edition; developer documents in English; the agent file (`AGENTS.md`) in English.
- Unknown host behaviour: measure it (build, run, smoke) before documenting it; write "unverified" otherwise.

### 2.4 Stop and park rules

- **Same CI failure three times**, or a fix that would need an owner decision outside §2.3: park the branch (leave the PR open as a draft with a comment saying why) and move on.
- **Disk below 10 GB free on `A:`** after cleaning caches: stop starting builds; docs-only work continues.
- **Anything that would break a rule in §2.2 "Not allowed"**: skip it and list it in the morning report.
- **Do not claim "verified" without running it.** Every PR states what was run and what was not.

### 2.5 Morning report

When done or when the owner returns: one message listing, per item, merged / parked / not started, the PR numbers, the local gate results, each decision taken from §2.3, and the questions that still need the owner. The decision log below carries the same facts for the repository.

### Decision log

| Date | Decision | Why |
|---|---|---|
| 2026-11-08 | Tag scheme `<directory>-v<semver>` (hyphen, not slash). | Unambiguous release URLs and non-overlapping trigger globs. |
| 2026-11-08 | Spec module names stay as drafted. | Owner delegated; refine per slice. |

## 3. Documentation track

Problem: the repository has hundreds of markdown files written per implementation unit (about 100 under `pluto-lib/docs`, about 260 under `tconstruct/docs`, validation receipts, reference dumps), old and new mixed, not readable as a project. Goal: few documents, current, written for people; per-feature detail lives in code (rustdoc, tests), not in prose.

| Step | What |
|---|---|
| D1 | **Inventory and policy.** A script lists every `.md`/receipt file with size, last change and who references it (tests, CI, other documents); a policy decides keep / merge / rewrite / archive for each class. Target structure: `README.md` (what it is, status, how to run); `docs/` with `architecture.md`, `development.md`, `releasing.md`, `roadmap.md`, `migration/` (plan and PR classification); `pluto-lib/README.md` + `pluto-lib/docs/` (design principles, specification, support matrix, generated hook map); `tconstruct/README.md` + one play guide; API reference generated by rustdoc. |
| D2 | **Remove.** Per-implementation notes, validation receipts and superseded design notes leave the tree (still in the archive tag `archive/pre-docs-cleanup`). Tests that read documents are changed to read the thing they really check, or removed if they only guard prose. |
| D3 | **Rewrite** the kept documents for a human reader: current facts only, short sections, no history, English with a Japanese edition for the user-facing ones. |
| D4 | **Keep it small.** CI check that no new per-implementation markdown appears (a size/count budget for `docs/` and `*/docs/`). |
